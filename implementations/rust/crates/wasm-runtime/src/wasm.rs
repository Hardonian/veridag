//! wasmtime-backed engine for Veridag Wasm guests (feature `wasmtime`).
//!
//! Runs real Wasm modules built against the Veridag host ABI. The guest MUST
//! export a `veridag_run() -> i32` function and import host functions from the
//! `env` module:
//!
//! - `host_read(ptr: i32, len: i32) -> i32` — caller writes the 32-byte
//!   `ObjectId` little-endian at `ptr`; the host copies the object bytes
//!   immediately after it (at `ptr+32`) and returns the byte length, or `-1`.
//! - `host_write(ptr: i32, len: i32, class: i32, cap_ptr: i32) -> i32` — write
//!   `len` bytes from `ptr+32` as an object of `class`, authorized by the
//!   32-byte `CapabilityId` at `cap_ptr`. Returns `0` / `-1`.
//! - `host_spend(amount: i64, cap_ptr: i32) -> i32` — authorized spend.
//! - `host_epoch() -> i64` — current epoch.
//! - `host_log(ptr: i32, len: i32) -> i32` — emit a debug log line.
//!
//! Metering is enforced by [`VeridagLimiter`] (a `ResourceLimiter`): the host
//! counts units and the limiter refuses further fuel past the budget, which
//! traps the guest deterministically.
//!
//! Build with `cargo build -p veridag-wasm-runtime --features wasmtime`. It is
//! intentionally excluded from the default/CI build so the heavy wasmtime
//! dependency tree never risks the deterministic release build.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use wasmtime::{Caller, Config, Engine, Linker, Module, ResourceLimiter, Store};

use crate::{CapabilitySet, Epoch, GuestError, HostAbi, Metering, ObjectId};
use veridag_protocol_types::CapabilityId;

/// Host state threaded through the Wasm store. Doubles as the [`ResourceLimiter`].
pub struct HostState {
    abi: NativeAbiOwned,
    metering: Metering,
}

/// Owned host ABI state (mirrors the borrow-split `NativeAbi` in the parent
/// module but owns its fields so it can live inside a `Store`).
struct NativeAbiOwned {
    store: HashMap<ObjectId, (Vec<u8>, u32)>,
    caps: CapabilitySet,
    logs: Vec<String>,
    units: u64,
}

impl HostAbi for NativeAbiOwned {
    fn read_object(&mut self, id: &ObjectId) -> Option<Vec<u8>> {
        self.store.get(id).map(|(d, _)| d.clone())
    }
    fn write_object(
        &mut self,
        id: ObjectId,
        data: Vec<u8>,
        object_class: u32,
        auth: CapabilityId,
    ) -> Result<(), GuestError> {
        if !self.caps.covers_object_class(&auth, object_class) {
            return Err(GuestError::CapabilityDenied(format!(
                "cap {:?} does not cover class {}",
                auth, object_class
            )));
        }
        self.store.insert(id, (data, object_class));
        Ok(())
    }
    fn spend(&mut self, amount: u64, auth: CapabilityId) -> Result<(), GuestError> {
        self.caps
            .authorize_spend(&auth, amount)
            .map(|_| ())
            .map_err(|e| GuestError::CapabilityDenied(e.to_string()))
    }
    fn epoch(&self) -> Epoch {
        self.caps.epoch
    }
    fn log(&mut self, msg: &str) -> Result<(), GuestError> {
        self.logs.push(msg.to_string());
        Ok(())
    }
}

impl HostState {
    fn tick(&mut self) -> Result<(), GuestError> {
        self.abi.units += 1;
        if self.abi.units > self.metering.max_units {
            return Err(GuestError::GasExceeded(self.abi.units));
        }
        Ok(())
    }
}

impl ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(self.abi.units <= self.metering.max_units)
    }
    fn table_growing(
        &mut self,
        _current: u32,
        _desired: u32,
        _maximum: Option<u32>,
    ) -> anyhow::Result<bool> {
        Ok(self.abi.units <= self.metering.max_units)
    }
}

/// Read a 32-byte id (ObjectId or CapabilityId) from guest memory at `ptr`.
fn read_32(caller: &mut Caller<'_, HostState>, ptr: i32) -> [u8; 32] {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("guest exports memory");
    let mut buf = [0u8; 32];
    mem.read(caller, ptr as usize, &mut buf).expect("id read");
    buf
}

/// Read object bytes from guest memory at `ptr+32` into a fresh vec.
fn read_bytes(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<Vec<u8>> {
    let mem = caller.get_export("memory").and_then(|e| e.into_memory())?;
    let mut buf = vec![0u8; len as usize];
    mem.read(caller, (ptr + 32) as usize, &mut buf).ok()?;
    Some(buf)
}

/// Write `bytes` into guest memory at `ptr`; returns false on overflow.
fn write_bytes(caller: &mut Caller<'_, HostState>, ptr: i32, bytes: &[u8]) -> bool {
    match caller.get_export("memory").and_then(|e| e.into_memory()) {
        Some(mem) => mem.write(caller, ptr as usize, bytes).is_ok(),
        None => false,
    }
}

/// Engine that instantiates and runs a Wasm guest.
pub struct WasmEngine {
    engine: Engine,
    metering: Metering,
    store: HashMap<ObjectId, (Vec<u8>, u32)>,
    caps: CapabilitySet,
    logs: Vec<String>,
}

impl WasmEngine {
    /// Build an engine.
    pub fn new(
        metering: Metering,
        store: HashMap<ObjectId, (Vec<u8>, u32)>,
        caps: CapabilitySet,
    ) -> Self {
        let mut config = Config::new();
        config.wasm_multi_memory(false);
        let engine = Engine::new(&config).expect("wasmtime engine");
        Self {
            engine,
            metering,
            store,
            caps,
            logs: Vec::new(),
        }
    }

    /// Compile `wasm_bytes` and run its `veridag_run` export, wiring the host
    /// ABI. Returns the guest's i32 result, or a [`GuestError`].
    pub fn run_guest(&mut self, wasm_bytes: &[u8]) -> Result<i32, GuestError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| GuestError::Trap(format!("compile: {e}")))?;

        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        self.link_host_fns(&mut linker)?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                abi: NativeAbiOwned {
                    store: std::mem::take(&mut self.store),
                    caps: std::mem::take(&mut self.caps),
                    logs: Vec::new(),
                    units: 0,
                },
                metering: self.metering,
            },
        );
        store.limiter(|s| s as &mut dyn ResourceLimiter);

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| GuestError::Trap(format!("instantiate: {e}")))?;
        let run = instance
            .get_typed_func::<(), i32>(&mut store, "veridag_run")
            .map_err(|e| GuestError::Trap(format!("no veridag_run: {e}")))?;
        let result = run
            .call(&mut store, ())
            .map_err(|e| GuestError::Trap(format!("call: {e}")))?;

        let state = store.into_data();
        self.store = state.abi.store;
        self.caps = state.abi.caps;
        self.logs = state.abi.logs;
        Ok(result)
    }

    fn link_host_fns(&self, linker: &mut Linker<HostState>) -> Result<(), GuestError> {
        linker
            .func_wrap(
                "env",
                "host_read",
                |mut caller: Caller<'_, HostState>, ptr: i32, _len: i32| -> i32 {
                    let _ = caller.data_mut().tick();
                    let id = ObjectId(read_32(&mut caller, ptr));
                    match caller.data_mut().abi.read_object(&id) {
                        Some(bytes) if write_bytes(&mut caller, ptr + 32, &bytes) => {
                            bytes.len() as i32
                        }
                        _ => -1,
                    }
                },
            )
            .map_err(|e| GuestError::Trap(e.to_string()))?;

        linker
            .func_wrap(
                "env",
                "host_write",
                |mut caller: Caller<'_, HostState>,
                 ptr: i32,
                 len: i32,
                 class: i32,
                 cap_ptr: i32|
                 -> i32 {
                    let _ = caller.data_mut().tick();
                    let id = ObjectId(read_32(&mut caller, ptr));
                    let cap = CapabilityId(read_32(&mut caller, cap_ptr));
                    let bytes = match read_bytes(&mut caller, ptr, len) {
                        Some(b) => b,
                        None => return -1,
                    };
                    match caller
                        .data_mut()
                        .abi
                        .write_object(id, bytes, class as u32, cap)
                    {
                        Ok(()) => 0,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| GuestError::Trap(e.to_string()))?;

        linker
            .func_wrap(
                "env",
                "host_spend",
                |mut caller: Caller<'_, HostState>, amount: i64, cap_ptr: i32| -> i32 {
                    let _ = caller.data_mut().tick();
                    let cap = CapabilityId(read_32(&mut caller, cap_ptr));
                    match caller.data_mut().abi.spend(amount as u64, cap) {
                        Ok(()) => 0,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| GuestError::Trap(e.to_string()))?;

        linker
            .func_wrap(
                "env",
                "host_epoch",
                |caller: Caller<'_, HostState>| -> i64 { caller.data().abi.epoch() as i64 },
            )
            .map_err(|e| GuestError::Trap(e.to_string()))?;

        linker
            .func_wrap(
                "env",
                "host_log",
                |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                    let _ = caller.data_mut().tick();
                    let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                        Some(m) => m,
                        None => return -1,
                    };
                    let mut buf = vec![0u8; len as usize];
                    if mem.read(&caller, ptr as usize, &mut buf).is_err() {
                        return -1;
                    }
                    let _ = caller.data_mut().abi.log(&String::from_utf8_lossy(&buf));
                    0
                },
            )
            .map_err(|e| GuestError::Trap(e.to_string()))?;

        Ok(())
    }

    /// Debug logs emitted by the guest.
    pub fn logs(&self) -> &[String] {
        &self.logs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use veridag_capabilities::Capability;
    use veridag_capabilities::CapabilityKind;
    use veridag_protocol_types::CapabilityId;

    // A WAT guest that logs "hello" and returns 42. wasmtime parses WAT text
    // directly, so no separate guest build toolchain is required to test the
    // host ABI end-to-end.
    const GUEST_WAT: &str = r#"
(module
  (import "env" "host_log" (func $log (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "hello")
  (func (export "veridag_run") (result i32)
    (drop (call $log (i32.const 0) (i32.const 5)))
    (i32.const 42)
  )
)
"#;

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

    #[test]
    fn wat_guest_runs_through_host_abi() {
        let caps = CapabilitySet::new(vec![spend_cap(1, 100)], 7);
        let mut engine = WasmEngine::new(Metering::default(), HashMap::new(), caps);
        let wasm = wat::parse_str(GUEST_WAT).expect("wat compiles");
        let result = engine.run_guest(&wasm).expect("guest runs");
        assert_eq!(result, 42);
        assert_eq!(engine.logs(), &["hello".to_string()]);
    }
}
