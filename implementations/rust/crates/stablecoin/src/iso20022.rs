//! ISO 20022 Financial Messaging Engine (spec 22).
//!
//! Provides turnkey interoperability between legacy core banking mainframes
//! (SWIFT / Fedwire / CHIPS) and the Veridag settlement substrate.
//!
//! Supports:
//! 1. Ingestion and validation of `pacs.008.001.08` (Financial Institutional Customer Credit Transfer).
//! 2. 1-basis-point (0.01%) consortium clearing surcharge calculation.
//! 3. Generation of ISO 20022 `pacs.002.001.10` (Payment Status Report) embedding
//!    cryptographic checkpoint IDs and state roots.

#![forbid(unsafe_code)]

use crate::{
    SettlerBatchSettlement, SettlerPayoutItem, SettlerReconciliationAnchor, StablecoinError,
    USDV_SCALE,
};
use thiserror::Error;
use veridag_protocol_types::{Address, Hash, ObjectId};

/// Errors produced during ISO 20022 parsing and message conversion.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Iso20022Error {
    /// Required XML tag is missing.
    #[error("missing required XML tag: {0}")]
    MissingTag(&'static str),
    /// Invalid numerical amount format.
    #[error("invalid amount string: {0}")]
    InvalidAmount(String),
    /// Unsupported currency.
    #[error("unsupported ISO currency: {0} (supported: USD, MXN, CAD)")]
    UnsupportedCurrency(String),
    /// Stablecoin engine execution error.
    #[error("settlement error: {0}")]
    Settlement(#[from] StablecoinError),
}

/// Parsed ISO 20022 `pacs.008.001.08` Credit Transfer instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pacs008CreditTransfer {
    /// Group header message ID.
    pub msg_id: String,
    /// Unique End-to-End identification.
    pub end_to_end_id: String,
    /// Unique End-to-End Transaction Reference (UUIDv4 format).
    pub uetr: String,
    /// Settlement currency (USD, MXN, CAD).
    pub currency: String,
    /// Total amount in micro-units (6 decimal places).
    pub amount_micro_units: u128,
    /// Debtor routing identifier / IBAN / BIC.
    pub debtor_id: String,
    /// Creditor routing identifier / IBAN / BIC.
    pub creditor_id: String,
    /// Debtor address mapped onto Veridag.
    pub debtor_address: Address,
    /// Creditor address mapped onto Veridag.
    pub creditor_address: Address,
}

/// Parse a standard ISO 20022 `pacs.008.001.08` XML string.
pub fn parse_pacs008_xml(
    xml: &str,
    debtor_address: Address,
    creditor_address: Address,
) -> Result<Pacs008CreditTransfer, Iso20022Error> {
    let msg_id = extract_tag(xml, "MsgId").ok_or(Iso20022Error::MissingTag("MsgId"))?;
    let end_to_end_id =
        extract_tag(xml, "EndToEndId").ok_or(Iso20022Error::MissingTag("EndToEndId"))?;
    let uetr = extract_tag(xml, "UETR").unwrap_or_else(|| end_to_end_id.clone());

    let currency = if let Some(amt_block) = extract_block(xml, "IntrBkSttlmAmt") {
        if let Some(pos) = amt_block.find("Ccy=\"") {
            let rest = &amt_block[pos + 5..];
            rest.split('"').next().unwrap_or("USD").to_string()
        } else {
            "USD".to_string()
        }
    } else {
        "USD".to_string()
    };

    if currency != "USD" && currency != "MXN" && currency != "CAD" {
        return Err(Iso20022Error::UnsupportedCurrency(currency));
    }

    let amt_str =
        extract_tag(xml, "IntrBkSttlmAmt").ok_or(Iso20022Error::MissingTag("IntrBkSttlmAmt"))?;
    let amount_micro_units = parse_decimal_to_micro(&amt_str)?;

    let debtor_id = extract_tag(xml, "DbtrAcct").unwrap_or_else(|| "DEBTOR-MAIN".into());
    let creditor_id = extract_tag(xml, "CdtrAcct").unwrap_or_else(|| "CREDITOR-MAIN".into());

    Ok(Pacs008CreditTransfer {
        msg_id,
        end_to_end_id,
        uetr,
        currency,
        amount_micro_units,
        debtor_id,
        creditor_id,
        debtor_address,
        creditor_address,
    })
}

/// Convert a `pacs.008` instruction into an atomic Veridag `SettlerBatchSettlement`.
///
/// Automatically deducts the 1-basis-point (0.01%) protocol clearing surcharge:
/// - 80% of the surcharge is allocated to validator staking rewards.
/// - 20% of the surcharge is credited to the consortium insurance reserve pool.
pub fn pacs008_to_settler_batch(
    transfer: &Pacs008CreditTransfer,
    source_bank_address: Address,
    validator_reward_pool: Address,
    insurance_reserve_pool: Address,
    batch_seq: u64,
) -> (SettlerBatchSettlement, u128) {
    let raw_amount = transfer.amount_micro_units;
    let total_fee = raw_amount / 10_000;

    let validator_fee = (total_fee * 80) / 100;
    let insurance_fee = total_fee.saturating_sub(validator_fee);
    let net_creditor_payout = raw_amount.saturating_sub(total_fee);

    let payouts = vec![
        SettlerPayoutItem {
            recipient: transfer.creditor_address,
            amount: net_creditor_payout,
            memo: [0u8; 32],
        },
        SettlerPayoutItem {
            recipient: validator_reward_pool,
            amount: validator_fee,
            memo: [1u8; 32],
        },
        SettlerPayoutItem {
            recipient: insurance_reserve_pool,
            amount: insurance_fee,
            memo: [2u8; 32],
        },
    ];

    let mut tenant_id = [0u8; 32];
    let source_bytes = transfer.debtor_id.as_bytes();
    let copy_len = source_bytes.len().min(32);
    tenant_id[..copy_len].copy_from_slice(&source_bytes[..copy_len]);

    let mut run_id = [0u8; 32];
    run_id[..8].copy_from_slice(&batch_seq.to_be_bytes());

    let mut manifest_hash = [0u8; 32];
    let e2e_bytes = transfer.end_to_end_id.as_bytes();
    let copy_e2e = e2e_bytes.len().min(32);
    manifest_hash[..copy_e2e].copy_from_slice(&e2e_bytes[..copy_e2e]);

    let anchor = SettlerReconciliationAnchor {
        tenant_id,
        run_id,
        manifest_hash,
        variance_summary_hash: [0u8; 32],
        total_settled_micro_units: raw_amount,
        transaction_count: 1,
        timestamp: 1726099200,
    };

    (
        SettlerBatchSettlement {
            source_account: source_bank_address,
            anchor,
            payouts,
        },
        total_fee,
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Generate an ISO 20022 `pacs.002.001.10` Payment Status Report XML confirmation.
pub fn generate_pacs002_xml(
    original_msg_id: &str,
    original_end_to_end_id: &str,
    original_uetr: &str,
    state_root: Hash,
    anchor_id: ObjectId,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
  <FIToFIPmtStsRpt>
    <GrpHdr>
      <MsgId>VERIDAG-CONF-{original_msg_id}</MsgId>
      <CreDtTm>2026-09-12T00:00:00Z</CreDtTm>
    </GrpHdr>
    <TxInfAndSts>
      <OrgnlEndToEndId>{original_end_to_end_id}</OrgnlEndToEndId>
      <OrgnlUETR>{original_uetr}</OrgnlUETR>
      <TxSts>ACSC</TxSts>
      <StsRsnInf>
        <Rsn>
          <Prtry>SETTLEMENT_FINALIZED_ON_DAG</Prtry>
        </Rsn>
        <AddtlInf>StateRoot:0x{state_root_hex}</AddtlInf>
        <AddtlInf>AnchorId:0x{anchor_id_hex}</AddtlInf>
      </StsRsnInf>
    </TxInfAndSts>
  </FIToFIPmtStsRpt>
</Document>"#,
        original_msg_id = original_msg_id,
        original_end_to_end_id = original_end_to_end_id,
        original_uetr = original_uetr,
        state_root_hex = hex_encode(&state_root),
        anchor_id_hex = hex_encode(&anchor_id.0)
    )
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_pos = xml.find(&open)?;
    let content_start = xml[start_pos..].find('>')? + start_pos + 1;
    let end_pos = xml[content_start..].find(&close)? + content_start;
    Some(xml[content_start..end_pos].trim().to_string())
}

fn extract_block(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_pos = xml.find(&open)?;
    let end_pos = xml[start_pos..].find(&close)? + start_pos + close.len();
    Some(xml[start_pos..end_pos].to_string())
}

fn parse_decimal_to_micro(s: &str) -> Result<u128, Iso20022Error> {
    let clean = s.trim();
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err(Iso20022Error::InvalidAmount(s.into()));
    }

    let whole: u128 = parts[0]
        .parse()
        .map_err(|_| Iso20022Error::InvalidAmount(s.into()))?;
    let whole_micro = whole
        .checked_mul(USDV_SCALE)
        .ok_or_else(|| Iso20022Error::InvalidAmount(s.into()))?;

    if parts.len() == 1 {
        return Ok(whole_micro);
    }

    let frac_str = parts[1];
    let frac_len = frac_str.len();
    let frac_val: u128 = frac_str
        .parse()
        .map_err(|_| Iso20022Error::InvalidAmount(s.into()))?;

    let frac_micro = if frac_len <= 6 {
        let multiplier = 10u128.pow(6 - frac_len as u32);
        frac_val * multiplier
    } else {
        let divisor = 10u128.pow(frac_len as u32 - 6);
        frac_val / divisor
    };

    whole_micro
        .checked_add(frac_micro)
        .ok_or_else(|| Iso20022Error::InvalidAmount(s.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StablecoinLedger;
    use veridag_object_state::ObjectState;

    #[test]
    fn test_pacs008_xml_parsing_and_surcharge() {
        let sample_xml = r#"
        <Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
          <FIToFICstmrCdtTrf>
            <GrpHdr>
              <MsgId>JPM-2026-USMCA-001</MsgId>
            </GrpHdr>
            <CdtTrfTxInf>
              <PmtId>
                <EndToEndId>E2E-USMCA-888999</EndToEndId>
                <UETR>c56a4180-65aa-42ec-a945-5fd21dec0538</UETR>
              </PmtId>
              <IntrBkSttlmAmt Ccy="USD">1000000.00</IntrBkSttlmAmt>
              <DbtrAcct>US-CITI-9988</DbtrAcct>
              <CdtrAcct>MX-BANORTE-7766</CdtrAcct>
            </CdtTrfTxInf>
          </FIToFICstmrCdtTrf>
        </Document>
        "#;

        let bank_a = [1u8; 32];
        let bank_b = [2u8; 32];
        let validator_pool = [3u8; 32];
        let insurance_pool = [4u8; 32];

        let tx = parse_pacs008_xml(sample_xml, bank_a, bank_b).expect("XML parse should succeed");
        assert_eq!(tx.msg_id, "JPM-2026-USMCA-001");
        assert_eq!(tx.end_to_end_id, "E2E-USMCA-888999");
        assert_eq!(tx.currency, "USD");
        assert_eq!(tx.amount_micro_units, 1_000_000 * USDV_SCALE);

        // Convert to Veridag Settler batch with automated 1 bps surcharge
        let (settler_batch, fee) =
            pacs008_to_settler_batch(&tx, bank_a, validator_pool, insurance_pool, 1);
        // 1 bps on 1,000,000 USD is 100 USD = 100,000,000 micro-units
        assert_eq!(fee, 100 * USDV_SCALE);
        assert_eq!(settler_batch.payouts[0].amount, 999_900 * USDV_SCALE); // Net recipient
        assert_eq!(settler_batch.payouts[1].amount, 80 * USDV_SCALE); // Validator pool (80%)
        assert_eq!(settler_batch.payouts[2].amount, 20 * USDV_SCALE); // Insurance reserve (20%)

        // Verify state execution
        let mut state = ObjectState::new();
        let mut ledger = StablecoinLedger::new();

        let custodian = veridag_crypto::Keypair::generate().unwrap();
        let attestation = crate::ReserveAttestation {
            oracle_id: custodian.address(),
            epoch: 1,
            timestamp: 1710000000,
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

        let minter_cap = veridag_capabilities::Capability {
            id: veridag_protocol_types::CapabilityId::ZERO,
            issuer: custodian.address(),
            holder: custodian.address(),
            kind: veridag_capabilities::CapabilityKind::ModifyObject {
                object_class: veridag_protocol_types::object_type::STABLECOIN,
            },
            constraints: veridag_capabilities::Constraints::default(),
            delegable: false,
            revoked: false,
            parent: None,
        };

        // Seed bank_a with 2,000,000 USD
        ledger
            .mint(
                &mut state,
                &custodian.address(),
                &bank_a,
                2_000_000 * USDV_SCALE,
                &minter_cap,
                1,
            )
            .unwrap();

        let receipt = ledger
            .execute_settler_batch(&mut state, &settler_batch)
            .expect("Batch execution must succeed");

        assert_eq!(receipt.balance_updates.len(), 4);

        // Generate pacs.002 receipt
        let dummy_root = [0xabu8; 32];
        let pacs002 = generate_pacs002_xml(
            &tx.msg_id,
            &tx.end_to_end_id,
            &tx.uetr,
            dummy_root,
            settler_batch.anchor.id(),
        );

        assert!(pacs002.contains("<TxSts>ACSC</TxSts>"));
        assert!(pacs002.contains("StateRoot:0xab"));
    }
}
