use crate::gc::GcStats;
use crate::value::{EntityId, TaskId, Value};

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
}

/// Trait for embedding the Writ runtime in a game engine or other host.
///
/// The host receives requests at transition points and returns responses.
/// One implementation per game engine.
pub trait RuntimeHost {
    /// Handle a request from the runtime. Return a response synchronously.
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse;

    /// Handle a log message from the runtime.
    fn on_log(&mut self, level: LogLevel, message: &str);

    /// Called after a garbage collection cycle completes.
    fn on_gc_complete(&mut self, _stats: &GcStats) {}
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
