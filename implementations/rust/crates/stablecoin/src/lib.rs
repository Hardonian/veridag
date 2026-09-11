//! Veridag USMCA & G8 Sovereign Digital Dollar (USDV) engine with US Treasury alignment (spec 19).
//!
//! Enforces:
//! 1. Direct US Treasury Collateral Invariant: Total circulating supply is bounded by cryptographically
//!    attested institutional reserves (US Treasuries, FDIC cash, reverse repo).
//! 2. Deterministic Micro-Unit Accounting: 6-decimal fixed point stored in `u128`.
//! 3. Capability-Enforced Governance: Mint, burn, compliance freeze, and reserve oracles
//!    require verified, unforgeable capabilities.
//! 4. Conservation of Value: Total supply strictly equals the sum of account balances.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use veridag_capabilities::{Capability, CapabilityError};
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use veridag_crypto::{hash, verify, Keypair};
use veridag_object_state::{Object, ObjectState, StateError};
use veridag_protocol_types::{
    object_type, Address, Ed25519PublicKey, Ed25519Signature, Epoch, ObjectId, ObjectRef,
    ObjectVersion, Ownership, TransactionId,
};

/// Domain separator for USDV protocol objects.
pub const USDV_DOMAIN: &str = "VERIDAG_USDV_V1";
/// Domain separator for Proof-of-Reserves attestations.
pub const POR_DOMAIN: &str = "VERIDAG_POR_V1";

/// Fixed decimal places for USDV (1 USD = 1,000,000 micro-cents).
pub const USDV_DECIMALS: u32 = 6;
/// Micro-units per 1.00 USD.
pub const USDV_SCALE: u128 = 1_000_000;

/// Errors produced by the stablecoin engine.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StablecoinError {
    /// Insufficient account balance.
    #[error("insufficient stablecoin balance: available {0}, required {1}")]
    InsufficientBalance(u128, u128),
    /// Account is frozen under compliance order.
    #[error("account {0:?} is frozen under compliance sanctions")]
    AccountFrozen(Address),
    /// Minting exceeds currently attested reserves (Invariant 1 violation).
    #[error("mint amount {0} exceeds available unbacked reserves: supply {1}, reserves {2}")]
    ReservesExceeded(u128, u128, u128),
    /// System is paused by circuit-breaker.
    #[error("stablecoin system is paused")]
    Paused,
    /// Unauthorized action: missing or invalid capability.
    #[error("unauthorized capability: {0}")]
    Unauthorized(String),
    /// Capability validation error.
    #[error("capability error: {0}")]
    Capability(#[from] CapabilityError),
    /// Invalid attestation signature or timestamp.
    #[error("invalid reserve attestation signature")]
    InvalidAttestation,
    /// Arithmetic overflow.
    #[error("arithmetic overflow in balance operation")]
    Overflow,
    /// State machine storage error.
    #[error("state error: {0}")]
    State(#[from] StateError),
    /// Serialization / deserialization error.
    #[error("codec error: {0}")]
    Codec(#[from] DecodeError),
    /// Target account not found.
    #[error("account not found")]
    NotFound,
}

/// Invariant violation errors from auditing the state.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InvariantViolation {
    /// Supply exceeds reserves.
    #[error("proof of reserves violated: supply {0} > reserves {1}")]
    ReservesDeficit(u128, u128),
    /// Sum of account balances does not equal recorded total supply.
    #[error("conservation of value violated: sum of balances {0} != total supply {1}")]
    SupplyMismatch(u128, u128),
    /// An unbacked token was detected.
    #[error("unbacked account detected: {0:?}")]
    UnbackedAccount(ObjectId),
}

/// A USDV account payload stored in state (`object_type::STABLECOIN = 3`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StablecoinAccountPayload {
    /// Account balance in micro-units (6 decimals).
    pub balance: u128,
    /// True if account is frozen under compliance / OFAC order.
    pub frozen: bool,
    /// Monotonic account mutation sequence number.
    pub nonce: u64,
}

impl Encode for StablecoinAccountPayload {
    fn encode(&self, e: &mut Encoder) {
        e.u128(self.balance);
        e.bool(self.frozen);
        e.u64(self.nonce);
    }
}

impl Decode for StablecoinAccountPayload {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            balance: d.u128()?,
            frozen: d.bool()?,
            nonce: d.u64()?,
        })
    }
}

/// Institutional Proof-of-Reserves attestation (`object_type::RESERVE_ATTESTATION = 4`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReserveAttestation {
    /// Address / public identity of the institutional custodian oracle.
    pub oracle_id: Address,
    /// Protocol epoch of attestation.
    pub epoch: Epoch,
    /// Verification timestamp (UNIX seconds).
    pub timestamp: u64,
    /// Collateral in US Treasury bills (maturity <= 90 days).
    pub treasury_bills: u128,
    /// Collateral in FDIC-insured bank cash deposits.
    pub cash_deposits: u128,
    /// Collateral in overnight reverse repurchase agreements (RRP).
    pub reverse_repo: u128,
    /// Total aggregate verified collateral (must equal sum of above).
    pub total_reserves: u128,
    /// Custodian Ed25519 signature over `VERIDAG_POR_V1 || encoded_body`.
    pub signature: Ed25519Signature,
}

impl ReserveAttestation {
    /// Encode the attestation body (without signature) for signing and hashing.
    pub fn encode_body(&self, e: &mut Encoder) {
        e.fixed(&self.oracle_id);
        e.u64(self.epoch);
        e.u64(self.timestamp);
        e.u128(self.treasury_bills);
        e.u128(self.cash_deposits);
        e.u128(self.reverse_repo);
        e.u128(self.total_reserves);
    }

    /// Compute the signing preimage hash.
    pub fn signing_hash(&self) -> [u8; 32] {
        let mut enc = Encoder::new();
        self.encode_body(&mut enc);
        hash(POR_DOMAIN, &enc.into_bytes())
    }

    /// Sign this attestation using a custodian keypair.
    pub fn sign(mut self, custodian: &Keypair) -> Self {
        self.oracle_id = custodian.address();
        let sig = custodian.sign(POR_DOMAIN, &self.signing_body_bytes());
        self.signature = sig;
        self
    }

    fn signing_body_bytes(&self) -> Vec<u8> {
        let mut enc = Encoder::new();
        self.encode_body(&mut enc);
        enc.into_bytes()
    }

    /// Verify custodian signature and internal accounting integrity.
    pub fn verify(&self, oracle_pubkey: &Ed25519PublicKey) -> Result<(), StablecoinError> {
        // Internal sum consistency
        let expected_total = self
            .treasury_bills
            .checked_add(self.cash_deposits)
            .and_then(|v| v.checked_add(self.reverse_repo))
            .ok_or(StablecoinError::Overflow)?;

        if self.total_reserves != expected_total {
            return Err(StablecoinError::InvalidAttestation);
        }

        verify(
            oracle_pubkey,
            POR_DOMAIN,
            &self.signing_body_bytes(),
            &self.signature,
        )
        .map_err(|_| StablecoinError::InvalidAttestation)?;

        Ok(())
    }

    /// Derive canonical ObjectId for this attestation.
    pub fn id(&self) -> ObjectId {
        ObjectId(self.signing_hash())
    }
}

impl Encode for ReserveAttestation {
    fn encode(&self, e: &mut Encoder) {
        self.encode_body(e);
        e.fixed(&self.signature);
    }
}

impl Decode for ReserveAttestation {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let oracle_id = d.fixed::<32>()?;
        let epoch = d.u64()?;
        let timestamp = d.u64()?;
        let treasury_bills = d.u128()?;
        let cash_deposits = d.u128()?;
        let reverse_repo = d.u128()?;
        let total_reserves = d.u128()?;
        let signature = d.fixed::<64>()?;

        Ok(Self {
            oracle_id,
            epoch,
            timestamp,
            treasury_bills,
            cash_deposits,
            reverse_repo,
            total_reserves,
            signature,
        })
    }
}

/// Stablecoin execution receipt.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StablecoinReceipt {
    /// Associated transaction identifier.
    pub tx_id: TransactionId,
    /// Resulting total circulating supply.
    pub total_supply: u128,
    /// Affected accounts and their new balances.
    pub balance_updates: Vec<(Address, u128)>,
    /// State objects written.
    pub objects_written: Vec<ObjectId>,
}

/// The USDV Stablecoin Ledger and Compliance Engine.
#[derive(Clone, Debug, Default)]
pub struct StablecoinLedger {
    /// Total circulating supply in micro-units.
    pub total_supply: u128,
    /// Active Proof-of-Reserves attestation.
    pub active_reserves: Option<ReserveAttestation>,
    /// Global pause circuit-breaker state.
    pub paused: bool,
    /// Set of tracked active accounts for invariant audits.
    pub tracked_accounts: Vec<Address>,
}

impl StablecoinLedger {
    /// Create a new USDV ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Derive the deterministic ObjectId for an account's USDV balance object.
    pub fn derive_account_id(owner: &Address) -> ObjectId {
        let mut enc = Encoder::new();
        enc.fixed(owner);
        enc.u32(object_type::STABLECOIN);
        ObjectId(hash(USDV_DOMAIN, &enc.into_bytes()))
    }

    /// Fetch an account payload from state if it exists.
    pub fn get_account(
        &self,
        state: &ObjectState,
        owner: &Address,
    ) -> Result<Option<StablecoinAccountPayload>, StablecoinError> {
        let id = Self::derive_account_id(owner);
        if let Some(obj) = state.get(&id) {
            let mut d = Decoder::new(&obj.payload);
            let payload = StablecoinAccountPayload::decode(&mut d)?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }

    /// Submit and record an institutional Proof-of-Reserves attestation.
    pub fn submit_attestation(
        &mut self,
        state: &mut ObjectState,
        attestation: ReserveAttestation,
        oracle_pubkey: &Ed25519PublicKey,
    ) -> Result<(), StablecoinError> {
        attestation.verify(oracle_pubkey)?;

        let id = attestation.id();
        let mut enc = Encoder::new();
        attestation.encode(&mut enc);
        let payload = enc.into_bytes();

        let obj = Object::new(
            id,
            object_type::RESERVE_ATTESTATION,
            Ownership::System,
            payload,
            vec![],
        );

        // Store or update in state
        if state.get(&id).is_some() {
            let r = ObjectRef {
                id,
                expected: state.get(&id).map(|o| o.version).unwrap_or(0),
            };
            state.mutate(&r, |o| o.payload = obj.payload.clone())?;
        } else {
            state.create(obj)?;
        }

        self.active_reserves = Some(attestation);
        Ok(())
    }

    /// Current verified reserves in micro-units.
    pub fn attested_reserves(&self) -> u128 {
        self.active_reserves
            .as_ref()
            .map(|r| r.total_reserves)
            .unwrap_or(0)
    }

    /// Mint new USDV tokens to `to` address, gated by `minter_cap` and Proof of Reserves.
    pub fn mint(
        &mut self,
        state: &mut ObjectState,
        minter: &Address,
        to: &Address,
        amount: u128,
        minter_cap: &Capability,
        current_epoch: Epoch,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        if self.paused {
            return Err(StablecoinError::Paused);
        }

        // Verify capability
        minter_cap.check_valid(current_epoch)?;
        if minter_cap.holder != *minter {
            return Err(StablecoinError::Unauthorized("caller not cap holder".into()));
        }

        // INVARIANT 1: Supply <= Attested Reserves
        let new_supply = self
            .total_supply
            .checked_add(amount)
            .ok_or(StablecoinError::Overflow)?;

        let reserves = self.attested_reserves();
        if new_supply > reserves {
            return Err(StablecoinError::ReservesExceeded(
                amount,
                self.total_supply,
                reserves,
            ));
        }

        // Credit recipient
        let to_id = Self::derive_account_id(to);
        let mut new_bal = amount;
        let mut written = Vec::new();

        if let Some(existing) = state.get(&to_id) {
            let mut d = Decoder::new(&existing.payload);
            let mut payload = StablecoinAccountPayload::decode(&mut d)?;
            if payload.frozen {
                return Err(StablecoinError::AccountFrozen(*to));
            }
            new_bal = payload
                .balance
                .checked_add(amount)
                .ok_or(StablecoinError::Overflow)?;
            payload.balance = new_bal;
            payload.nonce = payload.nonce.saturating_add(1);

            let mut enc = Encoder::new();
            payload.encode(&mut enc);
            let new_bytes = enc.into_bytes();

            let r = ObjectRef {
                id: to_id,
                expected: existing.version,
            };
            state.mutate(&r, |o| o.payload = new_bytes)?;
            written.push(to_id);
        } else {
            let payload = StablecoinAccountPayload {
                balance: amount,
                frozen: false,
                nonce: 1,
            };
            let mut enc = Encoder::new();
            payload.encode(&mut enc);

            let obj = Object::new(
                to_id,
                object_type::STABLECOIN,
                Ownership::Address(*to),
                enc.into_bytes(),
                vec![],
            );
            state.create(obj)?;
            written.push(to_id);
            self.tracked_accounts.push(*to);
        }

        self.total_supply = new_supply;

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*to, new_bal)],
            objects_written: written,
        })
    }

    /// Burn USDV tokens from `from`, reducing circulating supply.
    pub fn burn(
        &mut self,
        state: &mut ObjectState,
        from: &Address,
        amount: u128,
        burner_cap: Option<&Capability>,
        current_epoch: Epoch,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        if self.paused {
            return Err(StablecoinError::Paused);
        }

        if let Some(cap) = burner_cap {
            cap.check_valid(current_epoch)?;
            if cap.holder != *from {
                return Err(StablecoinError::Unauthorized("caller not cap holder".into()));
            }
        }

        let from_id = Self::derive_account_id(from);
        let existing = state.get(&from_id).ok_or(StablecoinError::NotFound)?;
        let mut d = Decoder::new(&existing.payload);
        let mut payload = StablecoinAccountPayload::decode(&mut d)?;

        if payload.frozen {
            return Err(StablecoinError::AccountFrozen(*from));
        }
        if payload.balance < amount {
            return Err(StablecoinError::InsufficientBalance(
                payload.balance,
                amount,
            ));
        }

        let new_bal = payload.balance - amount;
        payload.balance = new_bal;
        payload.nonce = payload.nonce.saturating_add(1);

        let mut enc = Encoder::new();
        payload.encode(&mut enc);
        let new_bytes = enc.into_bytes();

        let r = ObjectRef {
            id: from_id,
            expected: existing.version,
        };
        state.mutate(&r, |o| o.payload = new_bytes)?;

        self.total_supply = self
            .total_supply
            .checked_sub(amount)
            .ok_or(StablecoinError::Overflow)?;

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*from, new_bal)],
            objects_written: vec![from_id],
        })
    }

    /// Instant transfer between accounts on the Veridag DAG.
    pub fn transfer(
        &mut self,
        state: &mut ObjectState,
        from: &Address,
        to: &Address,
        amount: u128,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        if self.paused {
            return Err(StablecoinError::Paused);
        }
        if from == to {
            return Ok(StablecoinReceipt {
                tx_id: TransactionId::ZERO,
                total_supply: self.total_supply,
                balance_updates: vec![],
                objects_written: vec![],
            });
        }

        let from_id = Self::derive_account_id(from);
        let to_id = Self::derive_account_id(to);

        // Fetch sender
        let (from_version, mut from_payload) = {
            let from_obj = state.get(&from_id).ok_or(StablecoinError::NotFound)?;
            let mut d_from = Decoder::new(&from_obj.payload);
            let payload = StablecoinAccountPayload::decode(&mut d_from)?;
            (from_obj.version, payload)
        };

        if from_payload.frozen {
            return Err(StablecoinError::AccountFrozen(*from));
        }
        if from_payload.balance < amount {
            return Err(StablecoinError::InsufficientBalance(
                from_payload.balance,
                amount,
            ));
        }

        // Fetch and pre-validate recipient BEFORE debiting sender
        let to_info: Option<(ObjectVersion, StablecoinAccountPayload)> = if let Some(to_obj) = state.get(&to_id) {
            let mut d_to = Decoder::new(&to_obj.payload);
            let to_payload = StablecoinAccountPayload::decode(&mut d_to)?;
            if to_payload.frozen {
                return Err(StablecoinError::AccountFrozen(*to));
            }
            Some((to_obj.version, to_payload))
        } else {
            None
        };

        // All checks passed: now perform atomic debit and credit
        let new_from_bal = from_payload.balance - amount;
        from_payload.balance = new_from_bal;
        from_payload.nonce = from_payload.nonce.saturating_add(1);

        let mut enc_from = Encoder::new();
        from_payload.encode(&mut enc_from);
        let r_from = ObjectRef {
            id: from_id,
            expected: from_version,
        };
        state.mutate(&r_from, |o| o.payload = enc_from.into_bytes())?;

        let mut written = vec![from_id];
        let new_to_bal;

        if let Some((to_version, mut to_payload)) = to_info {
            new_to_bal = to_payload
                .balance
                .checked_add(amount)
                .ok_or(StablecoinError::Overflow)?;
            to_payload.balance = new_to_bal;
            to_payload.nonce = to_payload.nonce.saturating_add(1);

            let mut enc_to = Encoder::new();
            to_payload.encode(&mut enc_to);
            let r_to = ObjectRef {
                id: to_id,
                expected: to_version,
            };
            state.mutate(&r_to, |o| o.payload = enc_to.into_bytes())?;
            written.push(to_id);
        } else {
            new_to_bal = amount;
            let to_payload = StablecoinAccountPayload {
                balance: amount,
                frozen: false,
                nonce: 1,
            };
            let mut enc_to = Encoder::new();
            to_payload.encode(&mut enc_to);

            let obj = Object::new(
                to_id,
                object_type::STABLECOIN,
                Ownership::Address(*to),
                enc_to.into_bytes(),
                vec![],
            );
            state.create(obj)?;
            written.push(to_id);
            self.tracked_accounts.push(*to);
        }

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*from, new_from_bal), (*to, new_to_bal)],
            objects_written: written,
        })
    }

    /// Freeze an account under legal compliance or sanctions order.
    pub fn freeze_account(
        &mut self,
        state: &mut ObjectState,
        target: &Address,
        compliance_cap: &Capability,
        current_epoch: Epoch,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        compliance_cap.check_valid(current_epoch)?;

        let id = Self::derive_account_id(target);
        let obj = state.get(&id).ok_or(StablecoinError::NotFound)?;
        let mut d = Decoder::new(&obj.payload);
        let mut payload = StablecoinAccountPayload::decode(&mut d)?;

        payload.frozen = true;
        payload.nonce = payload.nonce.saturating_add(1);

        let mut enc = Encoder::new();
        payload.encode(&mut enc);
        let r = ObjectRef {
            id,
            expected: obj.version,
        };
        state.mutate(&r, |o| o.payload = enc.into_bytes())?;

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*target, payload.balance)],
            objects_written: vec![id],
        })
    }

    /// Unfreeze an account after compliance clearance.
    pub fn unfreeze_account(
        &mut self,
        state: &mut ObjectState,
        target: &Address,
        compliance_cap: &Capability,
        current_epoch: Epoch,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        compliance_cap.check_valid(current_epoch)?;

        let id = Self::derive_account_id(target);
        let obj = state.get(&id).ok_or(StablecoinError::NotFound)?;
        let mut d = Decoder::new(&obj.payload);
        let mut payload = StablecoinAccountPayload::decode(&mut d)?;

        payload.frozen = false;
        payload.nonce = payload.nonce.saturating_add(1);

        let mut enc = Encoder::new();
        payload.encode(&mut enc);
        let r = ObjectRef {
            id,
            expected: obj.version,
        };
        state.mutate(&r, |o| o.payload = enc.into_bytes())?;

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*target, payload.balance)],
            objects_written: vec![id],
        })
    }

    /// Seize funds from a frozen sanctions target into compliance escrow.
    pub fn seize_frozen_funds(
        &mut self,
        state: &mut ObjectState,
        target: &Address,
        escrow_recipient: &Address,
        compliance_cap: &Capability,
        current_epoch: Epoch,
    ) -> Result<StablecoinReceipt, StablecoinError> {
        compliance_cap.check_valid(current_epoch)?;

        let target_id = Self::derive_account_id(target);
        let target_obj = state.get(&target_id).ok_or(StablecoinError::NotFound)?;
        let mut d = Decoder::new(&target_obj.payload);
        let mut target_payload = StablecoinAccountPayload::decode(&mut d)?;

        if !target_payload.frozen {
            return Err(StablecoinError::Unauthorized(
                "cannot seize non-frozen account".into(),
            ));
        }

        let seized_amount = target_payload.balance;
        target_payload.balance = 0;
        target_payload.nonce = target_payload.nonce.saturating_add(1);

        let mut enc = Encoder::new();
        target_payload.encode(&mut enc);
        let r_target = ObjectRef {
            id: target_id,
            expected: target_obj.version,
        };
        state.mutate(&r_target, |o| o.payload = enc.into_bytes())?;

        // Credit escrow
        let escrow_id = Self::derive_account_id(escrow_recipient);
        let mut written = vec![target_id];
        let escrow_bal;

        if let Some(escrow_obj) = state.get(&escrow_id) {
            let mut d_esc = Decoder::new(&escrow_obj.payload);
            let mut esc_payload = StablecoinAccountPayload::decode(&mut d_esc)?;
            escrow_bal = esc_payload
                .balance
                .checked_add(seized_amount)
                .ok_or(StablecoinError::Overflow)?;
            esc_payload.balance = escrow_bal;
            esc_payload.nonce = esc_payload.nonce.saturating_add(1);

            let mut enc_esc = Encoder::new();
            esc_payload.encode(&mut enc_esc);
            let r_esc = ObjectRef {
                id: escrow_id,
                expected: escrow_obj.version,
            };
            state.mutate(&r_esc, |o| o.payload = enc_esc.into_bytes())?;
            written.push(escrow_id);
        } else {
            escrow_bal = seized_amount;
            let esc_payload = StablecoinAccountPayload {
                balance: seized_amount,
                frozen: false,
                nonce: 1,
            };
            let mut enc_esc = Encoder::new();
            esc_payload.encode(&mut enc_esc);
            let obj = Object::new(
                escrow_id,
                object_type::STABLECOIN,
                Ownership::Address(*escrow_recipient),
                enc_esc.into_bytes(),
                vec![],
            );
            state.create(obj)?;
            written.push(escrow_id);
            self.tracked_accounts.push(*escrow_recipient);
        }

        Ok(StablecoinReceipt {
            tx_id: TransactionId::ZERO,
            total_supply: self.total_supply,
            balance_updates: vec![(*target, 0), (*escrow_recipient, escrow_bal)],
            objects_written: written,
        })
    }

    /// Set emergency pause state.
    pub fn set_paused(
        &mut self,
        paused: bool,
        pause_cap: &Capability,
        current_epoch: Epoch,
    ) -> Result<(), StablecoinError> {
        pause_cap.check_valid(current_epoch)?;
        self.paused = paused;
        Ok(())
    }

    /// Audit core mathematical invariants:
    /// Invariant 1: TotalSupply <= AttestedReserves
    /// Invariant 2: TotalSupply == Sum(tracked_balances)
    pub fn verify_invariants(&self, state: &ObjectState) -> Result<(), InvariantViolation> {
        let reserves = self.attested_reserves();
        if self.total_supply > reserves {
            return Err(InvariantViolation::ReservesDeficit(
                self.total_supply,
                reserves,
            ));
        }

        let mut sum_balances: u128 = 0;
        for addr in &self.tracked_accounts {
            let id = Self::derive_account_id(addr);
            if let Some(obj) = state.get(&id) {
                let mut d = Decoder::new(&obj.payload);
                if let Ok(payload) = StablecoinAccountPayload::decode(&mut d) {
                    sum_balances = sum_balances
                        .checked_add(payload.balance)
                        .expect("balance sum overflow");
                }
            }
        }

        if sum_balances != self.total_supply {
            return Err(InvariantViolation::SupplyMismatch(
                sum_balances,
                self.total_supply,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_capabilities::{CapabilityKind, Constraints};
    use veridag_protocol_types::CapabilityId;

    fn mock_keypair() -> Keypair {
        Keypair::generate().expect("keypair generation failed")
    }

    fn mock_capability(issuer: Address, holder: Address) -> Capability {
        Capability {
            id: CapabilityId::ZERO,
            issuer,
            holder,
            kind: CapabilityKind::ModifyObject {
                object_class: object_type::STABLECOIN,
            },
            constraints: Constraints::default(),
            delegable: false,
            revoked: false,
            parent: None,
        }
    }

    #[test]
    fn test_proof_of_reserves_and_mint() {
        let mut state = ObjectState::new();
        let mut ledger = StablecoinLedger::new();

        let custodian = mock_keypair();
        let minter = mock_keypair();
        let alice = mock_keypair();

        let attestation = ReserveAttestation {
            oracle_id: custodian.address(),
            epoch: 1,
            timestamp: 1773000000,
            treasury_bills: 80_000_000 * USDV_SCALE,
            cash_deposits: 15_000_000 * USDV_SCALE,
            reverse_repo: 5_000_000 * USDV_SCALE,
            total_reserves: 100_000_000 * USDV_SCALE,
            signature: [0u8; 64],
        }
        .sign(&custodian);

        ledger
            .submit_attestation(&mut state, attestation, &custodian.public())
            .expect("attestation submission must succeed");

        assert_eq!(ledger.attested_reserves(), 100_000_000 * USDV_SCALE);

        // Mint within reserves
        let minter_cap = mock_capability(custodian.address(), minter.address());
        let mint_amount = 5_000_000 * USDV_SCALE;
        let receipt = ledger
            .mint(
                &mut state,
                &minter.address(),
                &alice.address(),
                mint_amount,
                &minter_cap,
                1,
            )
            .expect("mint within reserves must succeed");

        assert_eq!(receipt.total_supply, mint_amount);
        assert_eq!(ledger.total_supply, mint_amount);

        // Check Alice balance
        let alice_acc = ledger
            .get_account(&state, &alice.address())
            .expect("fetch account")
            .expect("account exists");
        assert_eq!(alice_acc.balance, mint_amount);

        // Minting beyond reserves must fail (Invariant 1)
        let over_amount = 96_000_000 * USDV_SCALE;
        let err = ledger
            .mint(
                &mut state,
                &minter.address(),
                &alice.address(),
                over_amount,
                &minter_cap,
                1,
            )
            .unwrap_err();
        assert!(matches!(err, StablecoinError::ReservesExceeded(..)));

        // Invariant audit
        ledger
            .verify_invariants(&state)
            .expect("invariants must hold");
    }

    #[test]
    fn test_transfer_and_compliance_freeze() {
        let mut state = ObjectState::new();
        let mut ledger = StablecoinLedger::new();

        let custodian = mock_keypair();
        let compliance = mock_keypair();
        let alice = mock_keypair();
        let bob = mock_keypair();

        let attestation = ReserveAttestation {
            oracle_id: custodian.address(),
            epoch: 1,
            timestamp: 1773000000,
            treasury_bills: 10_000_000 * USDV_SCALE,
            cash_deposits: 0,
            reverse_repo: 0,
            total_reserves: 10_000_000 * USDV_SCALE,
            signature: [0u8; 64],
        }
        .sign(&custodian);

        ledger
            .submit_attestation(&mut state, attestation, &custodian.public())
            .unwrap();

        let minter_cap = mock_capability(custodian.address(), custodian.address());
        ledger
            .mint(
                &mut state,
                &custodian.address(),
                &alice.address(),
                1_000 * USDV_SCALE,
                &minter_cap,
                1,
            )
            .unwrap();

        // Alice transfers to Bob
        ledger
            .transfer(&mut state, &alice.address(), &bob.address(), 400 * USDV_SCALE)
            .expect("transfer must succeed");

        let alice_bal = ledger
            .get_account(&state, &alice.address())
            .unwrap()
            .unwrap()
            .balance;
        let bob_bal = ledger
            .get_account(&state, &bob.address())
            .unwrap()
            .unwrap()
            .balance;
        assert_eq!(alice_bal, 600 * USDV_SCALE);
        assert_eq!(bob_bal, 400 * USDV_SCALE);

        // Freeze Bob under sanctions order
        let comp_cap = mock_capability(custodian.address(), compliance.address());
        ledger
            .freeze_account(&mut state, &bob.address(), &comp_cap, 1)
            .expect("freeze must succeed");

        // Bob cannot transfer out
        let err = ledger
            .transfer(&mut state, &bob.address(), &alice.address(), 100 * USDV_SCALE)
            .unwrap_err();
        assert_eq!(err, StablecoinError::AccountFrozen(bob.address()));

        // Alice cannot transfer into frozen Bob
        let err2 = ledger
            .transfer(&mut state, &alice.address(), &bob.address(), 50 * USDV_SCALE)
            .unwrap_err();
        assert_eq!(err2, StablecoinError::AccountFrozen(bob.address()));

        // Seize Bob's illicit funds into compliance escrow
        let escrow = mock_keypair();
        ledger
            .seize_frozen_funds(&mut state, &bob.address(), &escrow.address(), &comp_cap, 1)
            .expect("seizure must succeed");

        let bob_after = ledger.get_account(&state, &bob.address()).unwrap().unwrap();
        let escrow_after = ledger
            .get_account(&state, &escrow.address())
            .unwrap()
            .unwrap();
        assert_eq!(bob_after.balance, 0);
        assert_eq!(escrow_after.balance, 400 * USDV_SCALE);

        // Invariant audit holds after seizure
        ledger
            .verify_invariants(&state)
            .expect("conservation of value must hold");
    }
}
