use crate::gc::GcStats;
use crate::value::{EntityId, TaskId, Value};

/// Actions the host can request after a debug hook fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    /// Continue normal execution.
    Continue,
    /// Pause execution (breakpoint hit).
    Break,
    /// Step over: break when source line changes at same or lower call depth.
    StepOver,
    /// Step into: break when source line changes at any call depth.
    StepInto,
    /// Step out: break when current frame returns.
    StepOut,
    /// Disconnect debugger: clear all step state and resume without debug overhead.
    Disconnect,
}

/// Unique identifier for a pending host request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u32);

/// Log severity levels for RuntimeHost::on_log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Requests emitted by the runtime to the host at transition points.
///
/// Each variant carries the requesting `task_id` for host tracking.
#[derive(Debug, Clone)]
pub enum HostRequest {
    ExternCall {
        task_id: TaskId,
        extern_idx: u32,
        args: Vec<Value>,
        /// Pre-resolved human-readable string representations of `args`.
        ///
        /// FIX-03: The runtime fills this by resolving each arg through the GC heap before
        /// issuing the request. `Value::Ref` args are resolved to their string content;
        /// other values are formatted as strings. Hosts should use `display_args` for
        /// display purposes (e.g. say() output) rather than formatting `args` directly.
        display_args: Vec<String>,
    },
    EntitySpawn {
        task_id: TaskId,
        type_idx: u32,
    },
    FieldRead {
        task_id: TaskId,
        entity: EntityId,
        field_idx: u32,
    },
    FieldWrite {
        task_id: TaskId,
        entity: EntityId,
        field_idx: u32,
        value: Value,
    },
    GetComponent {
        task_id: TaskId,
        entity: EntityId,
        comp_type_idx: u32,
    },
    InitEntity {
        task_id: TaskId,
        entity: EntityId,
    },
    DestroyEntity {
        task_id: TaskId,
        entity: EntityId,
    },
    GetOrCreate {
        task_id: TaskId,
        type_idx: u32,
    },
    Join {
        task_id: TaskId,
        target_task: TaskId,
    },
}

/// Error returned by host when a request fails.
#[derive(Debug, Clone)]
pub enum HostError {
    NotSupported(String),
    Failed(String),
}

/// Response from the host to a runtime request.
#[derive(Debug, Clone)]
pub enum HostResponse {
    Value(Value),
    EntityHandle(EntityId),
    Confirmed,
    Error(HostError),
    /// Suspend the calling task until the host calls `Runtime::confirm()`.
    ///
    /// Use this for deferred/async operations where the result isn't available
    /// immediately (e.g., ECS queries that must wait for a safe sync point).
    /// The task will be parked and the scheduler moves on to other tasks.
    Suspend,
}

/// Trait for embedding the Writ runtime in a game engine or other host.
///
/// The host receives requests at transition points and returns responses.
/// One implementation per game engine.
pub trait RuntimeHost: Send + Sync {
    /// Handle a request from the runtime. Return a response synchronously.
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse;

    /// Handle a log message from the runtime.
    fn on_log(&mut self, level: LogLevel, message: &str);

    /// Handle an extern call with heap access. Returns None if not handled
    /// (falls through to on_request). Returns Some(response) if handled.
    ///
    /// Use for extern handlers that need to allocate Writ values on the heap.
    fn on_extern_call_with_heap(
        &mut self,
        _id: RequestId,
        _req: &HostRequest,
        _heap: &mut dyn crate::gc::GcHeap,
    ) -> Option<HostResponse> {
        None
    }

    /// Called after a garbage collection cycle completes.
    fn on_gc_complete(&mut self, _stats: &GcStats) {}

    /// Whether debug hooks should be called. Returns false by default.
    /// When false, the VM skips all debug hook calls for zero overhead.
    fn debug_enabled(&self) -> bool { false }

    /// Called before each instruction executes (only when debug_enabled() is true).
    /// Receives task ID, method index, program counter, and source location.
    /// Return DebugAction to control execution flow.
    fn before_instruction(
        &mut self,
        _task_id: TaskId,
        _method_idx: u32,
        _pc: u32,
        _source_line: u32,
        _source_col: u16,
    ) -> DebugAction { DebugAction::Continue }

    /// Called when a function is entered (only when debug_enabled() is true).
    fn on_function_enter(&mut self, _task_id: TaskId, _method_idx: u32) {}

    /// Called when a function is exited (only when debug_enabled() is true).
    fn on_function_exit(&mut self, _task_id: TaskId, _method_idx: u32) {}

    /// Called after the user module is parsed but before it is added to the Domain.
    ///
    /// The host may inspect attribute metadata via `view` and return `Err(reason)` to
    /// reject the module. A rejection causes `RuntimeBuilder::build` to return
    /// `RuntimeError::LoadError` containing the reason string.
    ///
    /// Does NOT fire for the virtual module or library modules — only for the user module.
    ///
    /// Default implementation accepts all modules unconditionally.
    fn on_module_load(&mut self, _view: &ModuleAttributeView<'_>) -> Result<(), String> {
        Ok(())
    }
}

/// A single attribute match returned by ModuleAttributeView queries.
///
/// Carries the decoded attribute arguments alongside the owner token so callers
/// can identify which definition the attribute was applied to.
#[derive(Debug, Clone)]
pub struct AttributeMatch {
    /// Attribute name (from the string heap).
    pub name: String,
    /// Decoded argument list (empty vec when the attribute has no arguments).
    pub args: Vec<writ_module::attr::AttrValue>,
    /// Metadata token of the owner (type, method, field, etc.).
    pub owner: writ_module::token::MetadataToken,
    /// Owner kind discriminant: 0 = type, 1 = method, 2 = field/global.
    /// Never 3 (declaration) — those are filtered out.
    pub owner_kind: u8,
}

/// Read-only view of a module's attribute metadata, passed to `on_module_load`.
///
/// Provides inspection of the `attribute_defs` table without any side effects.
/// Declaration rows (`owner_kind == ATTR_OWNER_KIND_DECL`) are always excluded
/// from query results.
pub struct ModuleAttributeView<'a> {
    module: &'a writ_module::Module,
}

impl<'a> ModuleAttributeView<'a> {
    /// Create a new view over the given module.
    pub fn new(module: &'a writ_module::Module) -> Self {
        ModuleAttributeView { module }
    }

    /// Return the module name from the string heap.
    pub fn module_name(&self) -> &str {
        writ_module::heap::read_string(&self.module.string_heap, self.module.header.module_name)
            .unwrap_or("<unknown>")
    }

    /// Return all attribute applications whose name matches `attr_name`.
    ///
    /// Declaration rows (`owner_kind == ATTR_OWNER_KIND_DECL`) are excluded.
    pub fn query_attributes(&self, attr_name: &str) -> Vec<AttributeMatch> {
        use writ_module::tables::ATTR_OWNER_KIND_DECL;

        self.module
            .attribute_defs
            .iter()
            .filter(|row| {
                row.owner_kind != ATTR_OWNER_KIND_DECL
                    && writ_module::heap::read_string(&self.module.string_heap, row.name)
                        .ok() == Some(attr_name)
            })
            .map(|row| self.build_match(row))
            .collect()
    }

    /// Return all attribute applications on the TypeDef at the given 0-based index.
    ///
    /// Declaration rows are excluded.
    pub fn query_attributes_on(&self, typedef_idx: usize) -> Vec<AttributeMatch> {
        use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};

        let target_row = (typedef_idx + 1) as u32; // convert 0-based to 1-based

        self.module
            .attribute_defs
            .iter()
            .filter(|row| {
                row.owner_kind != ATTR_OWNER_KIND_DECL
                    && row.owner.table_id() == TableId::TypeDef.as_u8()
                    && row.owner.row_index() == Some(target_row)
            })
            .map(|row| self.build_match(row))
            .collect()
    }

    /// Return the decoded arguments for the first attribute matching `attr_name`
    /// on the given owner token, or `None` if no match exists.
    ///
    /// Declaration rows are excluded.
    pub fn query_attribute_value(
        &self,
        owner_token: writ_module::token::MetadataToken,
        attr_name: &str,
    ) -> Option<Vec<writ_module::attr::AttrValue>> {
        use writ_module::tables::ATTR_OWNER_KIND_DECL;

        self.module
            .attribute_defs
            .iter()
            .find(|row| {
                row.owner_kind != ATTR_OWNER_KIND_DECL
                    && row.owner == owner_token
                    && writ_module::heap::read_string(&self.module.string_heap, row.name)
                        .ok() == Some(attr_name)
            })
            .map(|row| self.decode_args(row.value))
    }

    /// Build an AttributeMatch from a raw table row.
    fn build_match(&self, row: &writ_module::tables::AttributeDefRow) -> AttributeMatch {
        let name = writ_module::heap::read_string(&self.module.string_heap, row.name)
            .unwrap_or("<unknown>")
            .to_owned();
        let args = self.decode_args(row.value);
        AttributeMatch {
            name,
            args,
            owner: row.owner,
            owner_kind: row.owner_kind,
        }
    }

    /// Decode attribute args from a blob heap offset.
    ///
    /// Offset 0 means no args (null blob) — returns empty vec without calling read_blob.
    fn decode_args(&self, value_offset: u32) -> Vec<writ_module::attr::AttrValue> {
        if value_offset == 0 {
            return Vec::new();
        }
        match writ_module::heap::read_blob(&self.module.blob_heap, value_offset) {
            Ok(blob) => {
                writ_module::attr::decode_attr_args(blob).unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }
}

/// No-op host that auto-confirms all requests with default responses.
///
/// Tasks never actually suspend when using NullHost — all requests are
/// immediately resolved. Used for testing.
pub struct NullHost;

impl RuntimeHost for NullHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { .. } => HostResponse::Value(Value::Void),
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {
        // Silently drop all log messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::TaskId;

    // ── ModuleAttributeView tests ───────────────────────────────────────

    fn make_module_with_attrs() -> writ_module::Module {
        use writ_module::ModuleBuilder;
        use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL, TypeDefKind};
        use writ_module::token::MetadataToken;
        use writ_module::attr::{encode_attr_args, AttrValue};

        let mut b = ModuleBuilder::new("TestModule");
        // TypeDef "QuestGiver" at index 0 (row 1)
        b.add_type_def("QuestGiver", "", TypeDefKind::Struct, 0);
        let typedef_token = MetadataToken::new(TableId::TypeDef.as_u8(), 1);
        // Declaration row: owner_kind = ATTR_OWNER_KIND_DECL (should be filtered)
        b.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, "Quest", &[]);
        // Application row: owner_kind = 0 (type), points to typedef
        let encoded = encode_attr_args(&[AttrValue::String("Chapter1".into())]);
        b.add_attribute_def(typedef_token, 0, "Quest", &encoded);
        b.build()
    }

    #[test]
    fn module_attribute_view_module_name_returns_name() {
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        assert_eq!(view.module_name(), "TestModule");
    }

    #[test]
    fn query_attributes_excludes_declaration_rows() {
        use writ_module::attr::AttrValue;
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        let matches = view.query_attributes("Quest");
        // Only the application row should be returned, not the decl row
        assert_eq!(matches.len(), 1, "expected 1 application match, got {}", matches.len());
        assert_eq!(matches[0].name, "Quest");
        assert_eq!(matches[0].args, vec![AttrValue::String("Chapter1".into())]);
    }

    #[test]
    fn query_attributes_returns_empty_for_no_match() {
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        let matches = view.query_attributes("NonExistent");
        assert!(matches.is_empty());
    }

    #[test]
    fn query_attributes_on_typedef_returns_correct_rows() {
        use writ_module::attr::AttrValue;
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        // typedef index 0 = row 1 (1-based)
        let matches = view.query_attributes_on(0);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Quest");
        assert_eq!(matches[0].args, vec![AttrValue::String("Chapter1".into())]);
    }

    #[test]
    fn query_attributes_on_different_typedef_returns_empty() {
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        // typedef index 1 doesn't exist, should return empty
        let matches = view.query_attributes_on(1);
        assert!(matches.is_empty());
    }

    #[test]
    fn null_host_on_module_load_returns_ok() {
        let module = make_module_with_attrs();
        let view = ModuleAttributeView::new(&module);
        let mut host = NullHost;
        let result = host.on_module_load(&view);
        assert!(result.is_ok());
    }

    #[test]
    fn null_host_debug_enabled_returns_false() {
        let host = NullHost;
        assert!(!host.debug_enabled());
    }

    #[test]
    fn null_host_before_instruction_returns_continue() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let action = host.before_instruction(task_id, 0, 0, 1, 0);
        assert_eq!(action, DebugAction::Continue);
    }

    #[test]
    fn null_host_function_hooks_are_callable() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        host.on_function_enter(task_id, 0);
        host.on_function_exit(task_id, 0);
        // No panic == success
    }

    #[test]
    fn null_host_extern_call_returns_void() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let req = HostRequest::ExternCall {
            task_id,
            extern_idx: 0,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Void) => {}
            other => panic!("expected Value(Void), got {:?}", other),
        }
    }

    #[test]
    fn null_host_field_read_returns_int_zero() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let entity = crate::value::EntityId::new(0, 0);
        let req = HostRequest::FieldRead {
            task_id,
            entity,
            field_idx: 0,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Int(0)) => {}
            other => panic!("expected Value(Int(0)), got {:?}", other),
        }
    }

    #[test]
    fn null_host_entity_spawn_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let req = HostRequest::EntitySpawn {
            task_id,
            type_idx: 0,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn null_host_field_write_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let entity = crate::value::EntityId::new(0, 0);
        let req = HostRequest::FieldWrite {
            task_id,
            entity,
            field_idx: 0,
            value: Value::Int(42),
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn null_host_init_entity_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let entity = crate::value::EntityId::new(0, 0);
        let req = HostRequest::InitEntity { task_id, entity };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn null_host_destroy_entity_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let entity = crate::value::EntityId::new(0, 0);
        let req = HostRequest::DestroyEntity { task_id, entity };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn null_host_get_component_returns_void() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let entity = crate::value::EntityId::new(0, 0);
        let req = HostRequest::GetComponent {
            task_id,
            entity,
            comp_type_idx: 0,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Void) => {}
            other => panic!("expected Value(Void), got {:?}", other),
        }
    }

    #[test]
    fn null_host_get_or_create_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let req = HostRequest::GetOrCreate {
            task_id,
            type_idx: 0,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn null_host_join_returns_confirmed() {
        let mut host = NullHost;
        let task_id = TaskId::new(0, 0);
        let target = TaskId::new(1, 0);
        let req = HostRequest::Join {
            task_id,
            target_task: target,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }
}
