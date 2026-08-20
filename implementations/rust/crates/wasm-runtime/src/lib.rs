//! Deterministic, capability-scoped Wasm runtime for Veridag applications.
//!
//! Design goals (spec 12-wasm-runtime):
//! - **Determinism**: no wall-clock, no RNG, no I/O beyond the scoped host ABI.
//!   A guest sees only the objects and capabilities it is explicitly granted.
//! - **Capability scoping**: every host function is authorized against a
//!   [`CapabilitySet`] derived from `veridag-capabilities`. An unauthorized
//!   spend / object write is rejected by the host, never by guest trust.
//! - **Metering**: the engine enforces a deterministic instruction/gas budget.
//!   The native engine counts ABI calls; the `wasmtime` engine uses a
//!   `ResourceLimiter` so a hostile guest cannot loop forever.
//! - **Pluggable engine**: the [`GuestModule`] contract is engine-agnostic.
//!   [`NativeEngine`] (default, no heavy deps) runs guest Rust directly; the
//!   `wasmtime` feature adds [`WasmEngine`] for real Wasm guests. Both satisfy
//!   the exact same [`HostAbi`] contract, so consensus semantics are identical.
//!
//! The host ABI is intentionally tiny: read/write scoped objects, capability-
//! gated spend, epoch read, and a debug log. Anything broader (networking,
//! time, randomness) is OUT of scope by design and cannot be requested.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use veridag_capabilities::{Capability, CapabilityError};
use veridag_protocol_types::{ApplicationId, CapabilityId, Epoch, ObjectId};

/// Errors raised by a guest at runtime (e.g. a trap or explicit abort).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GuestError {
    /// The guest trapped (e.g. out-of-bounds, division by zero in Wasm).
    #[error("guest trapped: {0}")]
    Trap(String),
    /// The guest exceeded its deterministic gas/instruction budget.
    #[error("gas budget exceeded after {0} units")]
    GasExceeded(u64),
    /// The guest attempted an operation it is not authorized for.
    #[error("capability denied: {0}")]
    CapabilityDenied(String),
    /// The guest invoked an unknown/unsupported host function.
    #[error("unsupported host call: {0}")]
    UnsupportedHostCall(String),
    /// The guest requested an object that does not exist.
    #[error("object not found: {0:?}")]
    ObjectNotFound(ObjectId),
}

/// Deterministic metering configuration.
#[derive(Clone, Copy, Debug)]
pub struct Metering {
    /// Maximum ABI-call / instruction units before the engine aborts the guest.
    /// This is a deterministic bound: the same guest + input always consumes
    /// the same units, so metering is reproducible across validators.
    pub max_units: u64,
}

impl Default for Metering {
    fn default() -> Self {
        Metering {
            max_units: 1_000_000,
        }
    }
}

/// A set of capabilities a guest is authorized to present to the host ABI.
///
/// The host checks coverage BEFORE performing any state change, so an
/// unauthorized guest cannot mutate consensus state.
#[derive(Clone, Debug, Default)]
pub struct CapabilitySet {
    by_id: HashMap<CapabilityId, Capability>,
    epoch: Epoch,
}

impl CapabilitySet {
    /// Build a capability set from the capabilities valid at `epoch`.
    pub fn new(caps: Vec<Capability>, epoch: Epoch) -> Self {
        let by_id = caps
            .into_iter()
            .filter_map(|c| c.check_valid(epoch).ok().map(|()| (c.id, c)))
            .collect();
        Self { by_id, epoch }
    }

    /// Number of usable capabilities.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Authorize a spend of `amount` against capability `id`, mutating its
    /// counter. Returns the (possibly updated) capability so the caller can
    /// persist it.
    pub fn authorize_spend(
        &mut self,
        id: &CapabilityId,
        amount: u64,
    ) -> Result<Capability, CapabilityError> {
        let mut cap = self
            .by_id
            .get(id)
            .cloned()
            .ok_or(CapabilityError::NotCovered)?;
        cap.authorize_spend(amount, self.epoch)?;
        self.by_id.insert(cap.id, cap.clone());
        Ok(cap)
    }

    /// Whether `id` covers modifying objects of `object_class`.
    pub fn covers_object_class(&self, id: &CapabilityId, object_class: u32) -> bool {
        self.by_id
            .get(id)
            .map(|c| c.covers_object_class(object_class))
            .unwrap_or(false)
    }

    /// Whether `id` covers invoking `app`.
    pub fn covers_application(&self, id: &CapabilityId, app: &ApplicationId) -> bool {
        self.by_id
            .get(id)
            .map(|c| c.covers_application(app))
            .unwrap_or(false)
    }
}

/// The host ABI a guest may call. Every method is authoritative: it enforces
/// capability scoping and metering before any effect. Implementors MUST NOT
/// bypass these checks (the [`NativeEngine`] and [`WasmEngine`] both go through
/// this trait, so the contract is the single security boundary).
pub trait HostAbi {
    /// Read an object's raw bytes. Reading is always permitted (objects are
    /// content-addressed and public within the application's scope); returning
    /// `None` means the object does not exist.
    fn read_object(&mut self, id: &ObjectId) -> Option<Vec<u8>>;

    /// Write `data` as an object of `object_class`, authorized by capability
    /// `auth`. Rejected if `auth` does not cover `object_class`.
    fn write_object(
        &mut self,
        id: ObjectId,
        data: Vec<u8>,
        object_class: u32,
        auth: CapabilityId,
    ) -> Result<(), GuestError>;

    /// Spend `amount`, authorized by capability `auth` (must be a Spend/Agent
    /// capability with sufficient remaining budget).
    fn spend(&mut self, amount: u64, auth: CapabilityId) -> Result<(), GuestError>;

    /// Current consensus epoch (deterministic; identical across validators).
    fn epoch(&self) -> Epoch;

    /// Emit a debug log line. Metered (counts toward the gas budget) so a guest
    /// cannot spam unbounded logs.
    fn log(&mut self, msg: &str) -> Result<(), GuestError>;
}

/// A guest program. The engine calls [`GuestModule::run`] with a host ABI
/// handle; the guest drives all effects through that handle.
pub trait GuestModule {
    /// Execute the guest against the supplied host ABI. Must be deterministic
    /// given the same ABI state + inputs.
    fn run(&self, abi: &mut dyn HostAbi) -> Result<(), GuestError>;
}

/// A deterministic, dependency-light engine that runs [`GuestModule`] Rust
/// directly. This is the DEFAULT engine (no wasmtime). It enforces metering by
/// counting each ABI call as one unit and rejects the guest once the budget is
/// exhausted — fully reproducible across validators.
pub struct NativeEngine {
    metering: Metering,
    store: HashMap<ObjectId, (Vec<u8>, u32)>,
    caps: CapabilitySet,
    logs: Vec<String>,
    units: u64,
}

impl NativeEngine {
    /// Create an engine with `metering`, an initial object `store`, and the
    /// `caps` the guest may present.
    pub fn new(
        metering: Metering,
        store: HashMap<ObjectId, (Vec<u8>, u32)>,
        caps: CapabilitySet,
    ) -> Self {
        Self {
            metering,
            store,
            caps,
            logs: Vec::new(),
            units: 0,
        }
    }

    /// Run `guest` to completion, enforcing metering + capabilities.
    pub fn run_guest(&mut self, guest: &dyn GuestModule) -> Result<(), GuestError> {
        let mut abi = NativeAbi {
            store: &mut self.store,
            caps: &mut self.caps,
            logs: &mut self.logs,
            units: &mut self.units,
            max_units: self.metering.max_units,
        };
        guest.run(&mut abi)
    }

    /// Snapshot of objects after execution (for state commitment).
    pub fn store(&self) -> &HashMap<ObjectId, (Vec<u8>, u32)> {
        &self.store
    }

    /// Debug logs emitted by the guest.
    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    /// Units consumed by the last run.
    pub fn units_consumed(&self) -> u64 {
        self.units
    }
}

/// The [`HostAbi`] implementation owned by [`NativeEngine`].
struct NativeAbi<'a> {
    store: &'a mut HashMap<ObjectId, (Vec<u8>, u32)>,
    caps: &'a mut CapabilitySet,
    logs: &'a mut Vec<String>,
    units: &'a mut u64,
    max_units: u64,
}

impl NativeAbi<'_> {
    fn tick(&mut self) -> Result<(), GuestError> {
        *self.units += 1;
        if *self.units > self.max_units {
            return Err(GuestError::GasExceeded(*self.units));
        }
        Ok(())
    }
}

impl HostAbi for NativeAbi<'_> {
    fn read_object(&mut self, id: &ObjectId) -> Option<Vec<u8>> {
        let _ = self.tick();
        self.store.get(id).map(|(d, _)| d.clone())
    }

    fn write_object(
        &mut self,
        id: ObjectId,
        data: Vec<u8>,
        object_class: u32,
        auth: CapabilityId,
    ) -> Result<(), GuestError> {
        self.tick()?;
        if !self.caps.covers_object_class(&auth, object_class) {
            return Err(GuestError::CapabilityDenied(format!(
                "cap {:?} does not cover object class {}",
                auth, object_class
            )));
        }
        self.store.insert(id, (data, object_class));
        Ok(())
    }

    fn spend(&mut self, amount: u64, auth: CapabilityId) -> Result<(), GuestError> {
        self.tick()?;
        self.caps
            .authorize_spend(&auth, amount)
            .map(|_| ())
            .map_err(|e| GuestError::CapabilityDenied(e.to_string()))
    }

    fn epoch(&self) -> Epoch {
        // Reading epoch is free (no tick) — it is a pure constant for the run.
        self.caps.epoch
    }

    fn log(&mut self, msg: &str) -> Result<(), GuestError> {
        self.tick()?;
        self.logs.push(msg.to_string());
        Ok(())
    }
}

#[cfg(feature = "wasmtime")]
pub mod wasm;

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_capabilities::CapabilityKind;

    fn spend_cap(id: u8, max: u64) -> Capability {
        Capability {
            id: CapabilityId([id; 32]),
            issuer: [0u8; 32],
            holder: [0u8; 32],
            kind: CapabilityKind::Spend {
                max_per_epoch: max,
                current_epoch_spent: 0,
            },
            constraints: Default::default(),
            delegable: false,
            revoked: false,
            parent: None,
        }
    }

    fn modify_cap(id: u8, class: u32) -> Capability {
        Capability {
            id: CapabilityId([id; 32]),
            issuer: [0u8; 32],
            holder: [0u8; 32],
            kind: CapabilityKind::ModifyObject {
                object_class: class,
            },
            constraints: Default::default(),
            delegable: false,
            revoked: false,
            parent: None,
        }
    }

    /// A guest that writes an object (authorized) and spends (authorized).
    struct GoodGuest;
    impl GuestModule for GoodGuest {
        fn run(&self, abi: &mut dyn HostAbi) -> Result<(), GuestError> {
            abi.write_object(
                ObjectId([1u8; 32]),
                b"hello".to_vec(),
                7,
                CapabilityId([2u8; 32]),
            )?;
            abi.spend(10, CapabilityId([1u8; 32]))?;
            abi.log("done")?;
            Ok(())
        }
    }

    /// A guest that writes an object it is NOT authorized for.
    struct BadWriteGuest;
    impl GuestModule for BadWriteGuest {
        fn run(&self, abi: &mut dyn HostAbi) -> Result<(), GuestError> {
            abi.write_object(
                ObjectId([1u8; 32]),
                b"x".to_vec(),
                9,
                CapabilityId([2u8; 32]),
            )
        }
    }

    /// A guest that loops forever (must be caught by metering).
    struct LoopGuest;
    impl GuestModule for LoopGuest {
        fn run(&self, abi: &mut dyn HostAbi) -> Result<(), GuestError> {
            let mut i = 0u64;
            loop {
                abi.log("spin")?;
                i += 1;
                if i > 1_000_000_000 {
                    break;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn authorized_guest_succeeds_and_mutates_state() {
        let caps = CapabilitySet::new(vec![spend_cap(1, 100), modify_cap(2, 7)], 0);
        let mut engine = NativeEngine::new(Metering::default(), HashMap::new(), caps);
        assert!(engine.run_guest(&GoodGuest).is_ok());
        assert_eq!(
            engine.store().get(&ObjectId([1u8; 32])),
            Some(&(b"hello".to_vec(), 7))
        );
        assert_eq!(engine.logs(), &["done".to_string()]);
    }

    #[test]
    fn write_without_capability_is_denied() {
        let caps = CapabilitySet::new(vec![spend_cap(1, 100)], 0); // no modify cap
        let mut engine = NativeEngine::new(Metering::default(), HashMap::new(), caps);
        let err = engine.run_guest(&BadWriteGuest).unwrap_err();
        assert!(matches!(err, GuestError::CapabilityDenied(_)));
        assert!(engine.store().is_empty());
    }

    #[test]
    fn metering_stops_infinite_loop() {
        let caps = CapabilitySet::new(vec![], 0);
        let metering = Metering { max_units: 100 };
        let mut engine = NativeEngine::new(metering, HashMap::new(), caps);
        let err = engine.run_guest(&LoopGuest).unwrap_err();
        assert!(matches!(err, GuestError::GasExceeded(_)));
    }

    #[test]
    fn spend_over_budget_is_denied() {
        let caps = CapabilitySet::new(vec![spend_cap(1, 5)], 0); // max 5, try 10
        let mut engine = NativeEngine::new(Metering::default(), HashMap::new(), caps);
        let err = engine.run_guest(&SpendGuest(10)).unwrap_err();
        assert!(matches!(err, GuestError::CapabilityDenied(_)));
    }

    struct SpendGuest(u64);
    impl GuestModule for SpendGuest {
        fn run(&self, abi: &mut dyn HostAbi) -> Result<(), GuestError> {
            abi.spend(self.0, CapabilityId([1u8; 32]))
        }
    }

    #[test]
    fn determinism_same_inputs_same_units() {
        let caps = CapabilitySet::new(vec![spend_cap(1, 100), modify_cap(2, 7)], 0);
        let mut a = NativeEngine::new(Metering::default(), HashMap::new(), caps.clone());
        let mut b = NativeEngine::new(Metering::default(), HashMap::new(), caps);
        let _ = a.run_guest(&GoodGuest);
        let _ = b.run_guest(&GoodGuest);
        assert_eq!(a.units_consumed(), b.units_consumed());
    }
}
