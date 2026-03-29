//! Convenience layer for registering extern function handlers.
//!
//! Game engines typically provide extern functions (storage, UI, audio, etc.)
//! that Writ scripts call via `extern fn` declarations. `ExternRegistry` maps
//! extern names to handlers at load time, so the game doesn't need to
//! implement the full `RuntimeHost` trait from scratch.
//!
//! # ECS compatibility
//!
//! In ECS architectures, the world may not be accessible when the VM calls
//! an extern (the ECS might be mid-system-tick, or the world borrow is held).
//! The registry supports two handler modes:
//!
//! - **Immediate**: The handler runs inline and returns a value. Use for pure
//!   queries or cheap operations that don't touch the ECS world.
//!
//! - **Deferred**: The call is buffered into a command queue. The game drains
//!   the queue during a safe ECS phase and confirms each request via
//!   `Runtime::confirm()`. The calling task suspends until confirmed.
//!
//! # Example
//!
//! ```ignore
//! let mut registry = ExternRegistry::new();
//!
//! // Immediate: pure lookup, no world mutation
//! registry.register("storage_get", ExternHandler::Immediate(Box::new(|args| {
//!     Ok(Value::Int(0)) // placeholder
//! })));
//!
//! // Deferred: queued for the next ECS sync point
//! registry.register("ui_rect", ExternHandler::Deferred);
//!
//! // Wrap an existing host (e.g., for entity/lifecycle handling)
//! let host = registry.build(module);
//! let rt = RuntimeBuilder::new(module).with_host(host).build()?;
//! ```

use rustc_hash::FxHashMap;
use writ_module::Module;
use writ_module::heap::read_string;

use crate::gc::GcHeap;
use crate::host::{HostRequest, HostResponse, LogLevel, RequestId, RuntimeHost};
use crate::value::{TaskId, Value};

/// Handler for a single extern function.
pub enum ExternHandler {
    /// Run inline and return a value immediately. The task does not suspend.
    ///
    /// Use for pure queries, cheap computations, or anything that doesn't
    /// require ECS world access.
    Immediate(Box<dyn FnMut(&[Value]) -> Result<Value, String> + Send + Sync>),

    /// Run inline with heap access and return a value immediately.
    ///
    /// Use when the handler needs to allocate Writ values on the GC heap
    /// (strings, structs) to return to the script.
    ImmediateWithHeap(Box<dyn FnMut(&[Value], &mut dyn GcHeap) -> Result<Value, String> + Send + Sync>),

    /// Buffer the call for later confirmation. The task suspends until the
    /// game calls `Runtime::confirm()` with the result.
    ///
    /// Use for operations that mutate ECS state or must wait for a specific
    /// system phase.
    Deferred,
}

/// Pre-built extern dispatch table for a specific module.
///
/// Created by `ExternRegistry::build()`, which resolves extern names at
/// load time so runtime dispatch is O(1) by token index.
pub struct ExternHost {
    /// Keyed by 0-based ExternDef row index (decoded from the extern_idx token).
    handlers: FxHashMap<usize, ExternHandler>,
    /// Extern name table for diagnostics (parallel to module.extern_defs).
    extern_names: Vec<String>,
    /// Deferred calls waiting for the game to drain them.
    deferred_queue: Vec<DeferredCall>,
    /// Log handler (optional override).
    log_handler: Option<Box<dyn FnMut(LogLevel, &str) + Send + Sync>>,
    /// Fallback for non-extern HostRequests (entity spawn, field read, etc.).
    /// If None, entity/lifecycle requests return default confirmations.
    entity_handler: Option<Box<dyn FnMut(RequestId, &HostRequest) -> HostResponse + Send + Sync>>,
}

/// A buffered extern call waiting for the game to process it.
#[derive(Debug, Clone)]
pub struct DeferredCall {
    /// The runtime's request ID — pass this to `Runtime::confirm()`.
    pub request_id: RequestId,
    /// Task that is suspended waiting for this result.
    pub task_id: TaskId,
    /// The extern function name.
    pub name: String,
    /// Arguments passed by the script.
    pub args: Vec<Value>,
    /// Pre-resolved display strings for the arguments.
    pub display_args: Vec<String>,
}

/// Builder for constructing an `ExternHost`.
///
/// Register handlers by name, then call `build()` with the module to
/// resolve names to token indices.
pub struct ExternRegistry {
    handlers: FxHashMap<String, ExternHandler>,
    log_handler: Option<Box<dyn FnMut(LogLevel, &str) + Send + Sync>>,
    entity_handler: Option<Box<dyn FnMut(RequestId, &HostRequest) -> HostResponse + Send + Sync>>,
}

impl ExternRegistry {
    pub fn new() -> Self {
        Self {
            handlers: FxHashMap::default(),
            log_handler: None,
            entity_handler: None,
        }
    }

    /// Register a handler for an extern function by name.
    pub fn register(&mut self, name: &str, handler: ExternHandler) -> &mut Self {
        self.handlers.insert(name.to_string(), handler);
        self
    }

    /// Register an immediate handler using a closure shorthand.
    pub fn on(&mut self, name: &str, f: impl FnMut(&[Value]) -> Result<Value, String> + Send + Sync + 'static) -> &mut Self {
        self.handlers.insert(name.to_string(), ExternHandler::Immediate(Box::new(f)));
        self
    }

    /// Register an immediate handler with heap access using a closure shorthand.
    pub fn on_with_heap(&mut self, name: &str, f: impl FnMut(&[Value], &mut dyn GcHeap) -> Result<Value, String> + Send + Sync + 'static) -> &mut Self {
        self.handlers.insert(name.to_string(), ExternHandler::ImmediateWithHeap(Box::new(f)));
        self
    }

    /// Mark an extern as deferred (ECS-safe). Calls will be buffered.
    pub fn defer(&mut self, name: &str) -> &mut Self {
        self.handlers.insert(name.to_string(), ExternHandler::Deferred);
        self
    }

    /// Set a custom log handler. If not set, logs are silently dropped.
    pub fn with_log_handler(
        &mut self,
        handler: impl FnMut(LogLevel, &str) + Send + Sync + 'static,
    ) -> &mut Self {
        self.log_handler = Some(Box::new(handler));
        self
    }

    /// Set a handler for entity/lifecycle requests (EntitySpawn, FieldRead, etc.).
    /// If not set, these return default confirmations.
    pub fn with_entity_handler(
        &mut self,
        handler: impl FnMut(RequestId, &HostRequest) -> HostResponse + Send + Sync + 'static,
    ) -> &mut Self {
        self.entity_handler = Some(Box::new(handler));
        self
    }

    /// Validate that all extern functions declared in the module have registered handlers.
    ///
    /// Returns a list of unhandled extern names. An empty vec means all externs are covered.
    /// Built-in externs (`say`, `say_localized`, `choice`, `log::*`) are excluded from
    /// validation since the runtime handles them internally.
    pub fn validate(&self, module: &Module) -> Vec<String> {
        let builtins = [
            "say", "say_localized", "choice",
            "log::trace", "log::debug", "log::info", "log::warn", "log::error",
        ];
        let mut missing = Vec::new();
        for ed in &module.extern_defs {
            if let Ok(name) = read_string(&module.string_heap, ed.name) {
                if !builtins.contains(&name) && !self.handlers.contains_key(name) {
                    missing.push(name.to_string());
                }
            }
        }
        missing
    }

    /// Build the `ExternHost` by resolving registered names against the module's
    /// ExternDef table. Unregistered externs fall through to default behavior
    /// (return `Value::Void`).
    pub fn build(mut self, module: &Module) -> ExternHost {
        let mut handlers = FxHashMap::default();
        let mut extern_names = Vec::with_capacity(module.extern_defs.len());

        for (idx, ed) in module.extern_defs.iter().enumerate() {
            let name = read_string(&module.string_heap, ed.name)
                .unwrap_or("?")
                .to_string();
            if let Some(handler) = self.handlers.remove(&name) {
                handlers.insert(idx, handler);
            }
            extern_names.push(name);
        }

        ExternHost {
            handlers,
            extern_names,
            deferred_queue: Vec::new(),
            log_handler: self.log_handler,
            entity_handler: self.entity_handler,
        }
    }
}

impl Default for ExternRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternHost {
    /// Drain all deferred calls. The game should process these during a safe
    /// ECS phase and call `Runtime::confirm()` for each.
    pub fn drain_deferred(&mut self) -> Vec<DeferredCall> {
        std::mem::take(&mut self.deferred_queue)
    }

    /// Check whether there are pending deferred calls.
    pub fn has_deferred(&self) -> bool {
        !self.deferred_queue.is_empty()
    }

    /// Resolve an extern_idx MetadataToken to a 0-based row index.
    fn decode_extern_idx(extern_idx: u32) -> usize {
        let row_1based = (extern_idx & 0x00FF_FFFF) as usize;
        row_1based.saturating_sub(1)
    }

    /// Get the name for an extern index.
    pub fn extern_name(&self, extern_idx: u32) -> &str {
        let idx = Self::decode_extern_idx(extern_idx);
        self.extern_names.get(idx).map(|s| s.as_str()).unwrap_or("?")
    }
}

impl RuntimeHost for ExternHost {
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, args, display_args, task_id } => {
                let idx = Self::decode_extern_idx(*extern_idx);
                let name = self.extern_names.get(idx).cloned().unwrap_or_default();

                // Check for built-in log calls first
                match name.as_str() {
                    "log::trace" => { self.on_log(LogLevel::Trace, display_args.first().map(|s| s.as_str()).unwrap_or("")); return HostResponse::Value(Value::Void); }
                    "log::debug" => { self.on_log(LogLevel::Debug, display_args.first().map(|s| s.as_str()).unwrap_or("")); return HostResponse::Value(Value::Void); }
                    "log::info"  => { self.on_log(LogLevel::Info,  display_args.first().map(|s| s.as_str()).unwrap_or("")); return HostResponse::Value(Value::Void); }
                    "log::warn"  => { self.on_log(LogLevel::Warn,  display_args.first().map(|s| s.as_str()).unwrap_or("")); return HostResponse::Value(Value::Void); }
                    "log::error" => { self.on_log(LogLevel::Error, display_args.first().map(|s| s.as_str()).unwrap_or("")); return HostResponse::Value(Value::Void); }
                    _ => {}
                }

                // Registered handler dispatch
                if let Some(handler) = self.handlers.get_mut(&idx) {
                    match handler {
                        ExternHandler::Immediate(f) => {
                            match f(args) {
                                Ok(val) => HostResponse::Value(val),
                                Err(msg) => HostResponse::Error(
                                    crate::host::HostError::Failed(msg),
                                ),
                            }
                        }
                        ExternHandler::ImmediateWithHeap(_f) => {
                            // ImmediateWithHeap needs heap access — handled via
                            // on_extern_call_with_heap in the dispatch loop.
                            // If we reach here, the dispatch loop didn't call
                            // on_extern_call_with_heap first. Fall through to deferred.
                            HostResponse::Error(
                                crate::host::HostError::Failed(
                                    "ImmediateWithHeap handler requires heap-aware dispatch".to_string(),
                                ),
                            )
                        }
                        ExternHandler::Deferred => {
                            self.deferred_queue.push(DeferredCall {
                                request_id: id,
                                task_id: *task_id,
                                name,
                                args: args.clone(),
                                display_args: display_args.clone(),
                            });
                            // Suspend the task — the game drains deferred calls during
                            // a safe ECS phase and calls Runtime::confirm() with each result.
                            HostResponse::Suspend
                        }
                    }
                } else {
                    // Unregistered extern — return Void (silent fallthrough)
                    HostResponse::Value(Value::Void)
                }
            }

            // Entity/lifecycle requests — delegate to entity_handler or use defaults
            req => {
                if let Some(ref mut handler) = self.entity_handler {
                    handler(id, req)
                } else {
                    // Default confirmations matching NullHost behavior
                    match req {
                        HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
                        HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
                        _ => HostResponse::Confirmed,
                    }
                }
            }
        }
    }

    fn on_extern_call_with_heap(
        &mut self,
        _id: RequestId,
        req: &HostRequest,
        heap: &mut dyn GcHeap,
    ) -> Option<HostResponse> {
        if let HostRequest::ExternCall { extern_idx, args, .. } = req {
            let idx = Self::decode_extern_idx(*extern_idx);
            if let Some(handler) = self.handlers.get_mut(&idx) {
                if let ExternHandler::ImmediateWithHeap(f) = handler {
                    return Some(match f(args, heap) {
                        Ok(val) => HostResponse::Value(val),
                        Err(msg) => HostResponse::Error(
                            crate::host::HostError::Failed(msg),
                        ),
                    });
                }
            }
        }
        None
    }

    fn on_log(&mut self, level: LogLevel, message: &str) {
        if let Some(ref mut handler) = self.log_handler {
            handler(level, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::heap::intern_string;

    fn module_with_externs(names: &[&str]) -> Module {
        let mut m = Module::new();
        for name in names {
            let offset = intern_string(&mut m.string_heap, name);
            m.extern_defs.push(writ_module::tables::ExternDefRow {
                name: offset,
                signature: 0,
                import_name: 0,
                flags: 0,
            });
        }
        m
    }

    #[test]
    fn validate_reports_missing_externs() {
        let module = module_with_externs(&["storage_get", "storage_set", "ui_rect"]);
        let mut reg = ExternRegistry::new();
        reg.on("storage_get", |_| Ok(Value::Int(0)));
        // storage_set and ui_rect not registered
        let missing = reg.validate(&module);
        assert_eq!(missing, vec!["storage_set", "ui_rect"]);
    }

    #[test]
    fn validate_ignores_builtins() {
        let module = module_with_externs(&["say", "log::info", "storage_get"]);
        let mut reg = ExternRegistry::new();
        reg.on("storage_get", |_| Ok(Value::Int(0)));
        let missing = reg.validate(&module);
        assert!(missing.is_empty(), "builtins should not be flagged, got: {:?}", missing);
    }

    #[test]
    fn immediate_handler_returns_value() {
        let module = module_with_externs(&["get_health"]);
        let mut reg = ExternRegistry::new();
        reg.on("get_health", |_args| Ok(Value::Int(100)));
        let mut host = reg.build(&module);

        // ExternDef table_id=16, row=1 => token 0x10000001
        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Value(Value::Int(100)) => {}
            other => panic!("expected Value(Int(100)), got {:?}", other),
        }
    }

    #[test]
    fn immediate_handler_receives_args() {
        let module = module_with_externs(&["add"]);
        let mut reg = ExternRegistry::new();
        reg.on("add", |args| {
            let a = match args.get(0) { Some(Value::Int(n)) => *n, _ => 0 };
            let b = match args.get(1) { Some(Value::Int(n)) => *n, _ => 0 };
            Ok(Value::Int(a + b))
        });
        let mut host = reg.build(&module);

        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![Value::Int(3), Value::Int(7)],
            display_args: vec![],
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Value(Value::Int(10)) => {}
            other => panic!("expected Value(Int(10)), got {:?}", other),
        }
    }

    #[test]
    fn deferred_handler_queues_call() {
        let module = module_with_externs(&["ui_rect"]);
        let mut reg = ExternRegistry::new();
        reg.defer("ui_rect");
        let mut host = reg.build(&module);

        assert!(!host.has_deferred());

        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![Value::Float(10.0), Value::Float(20.0)],
            display_args: vec!["10.0".into(), "20.0".into()],
        };
        let resp = host.on_request(RequestId(42), &req);
        assert!(matches!(resp, HostResponse::Suspend), "deferred should return Suspend");

        assert!(host.has_deferred());
        let calls = host.drain_deferred();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "ui_rect");
        assert_eq!(calls[0].request_id, RequestId(42));
        assert!(!host.has_deferred());
    }

    #[test]
    fn unregistered_extern_returns_void() {
        let module = module_with_externs(&["unknown_fn"]);
        let reg = ExternRegistry::new();
        let mut host = reg.build(&module);

        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Value(Value::Void) => {}
            other => panic!("expected Value(Void), got {:?}", other),
        }
    }

    #[test]
    fn error_handler_returns_host_error() {
        let module = module_with_externs(&["fail_fn"]);
        let mut reg = ExternRegistry::new();
        reg.on("fail_fn", |_| Err("something broke".into()));
        let mut host = reg.build(&module);

        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Error(_) => {}
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn entity_requests_use_default_without_handler() {
        let module = module_with_externs(&[]);
        let reg = ExternRegistry::new();
        let mut host = reg.build(&module);

        let req = HostRequest::EntitySpawn {
            task_id: TaskId::new(0, 0),
            type_idx: 5,
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn entity_handler_override() {
        let module = module_with_externs(&[]);
        let mut reg = ExternRegistry::new();
        reg.with_entity_handler(|_id, req| {
            match req {
                HostRequest::EntitySpawn { .. } => {
                    HostResponse::EntityHandle(crate::value::EntityId::new(42, 0))
                }
                _ => HostResponse::Confirmed,
            }
        });
        let mut host = reg.build(&module);

        let req = HostRequest::EntitySpawn {
            task_id: TaskId::new(0, 0),
            type_idx: 5,
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::EntityHandle(eid) => assert_eq!(eid.index, 42),
            other => panic!("expected EntityHandle, got {:?}", other),
        }
    }

    #[test]
    fn build_with_multiple_externs_resolves_by_name() {
        // Two modules declare overlapping externs — the registry resolves by name
        let module = module_with_externs(&["storage_get", "storage_set", "ui_rect"]);
        let mut reg = ExternRegistry::new();
        reg.on("storage_get", |_| Ok(Value::Int(42)));
        reg.on("ui_rect", |_| Ok(Value::Int(99)));
        // storage_set not registered — will return Void
        let mut host = reg.build(&module);

        // storage_get is row 0 → token (16 << 24) | 1
        let tok_get: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: tok_get,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(1), &req) {
            HostResponse::Value(Value::Int(42)) => {}
            other => panic!("expected 42, got {:?}", other),
        }

        // ui_rect is row 2 → token (16 << 24) | 3
        let tok_rect: u32 = (16u32 << 24) | 3;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: tok_rect,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(2), &req) {
            HostResponse::Value(Value::Int(99)) => {}
            other => panic!("expected 99, got {:?}", other),
        }
    }
}
