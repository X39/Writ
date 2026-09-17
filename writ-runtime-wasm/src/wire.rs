use serde::{Deserialize, Serialize};
use writ_runtime::host::HostError;
use writ_runtime::{EntityId, HostRequest, HostResponse, Runtime, RuntimeHost, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WireValue {
    Void,
    Int {
        value: String,
    },
    Float {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Entity {
        index: u32,
        generation: u32,
    },
    /// Heap objects other than strings deliberately remain opaque at the JS boundary.
    Opaque {
        display: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WireHandle {
    pub index: u32,
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WireResponse {
    Value { value: WireValue },
    Entity { index: u32, generation: u32 },
    Confirmed,
    Deferred,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WireRequest {
    pub id: u32,
    pub task: WireHandle,
    #[serde(flatten)]
    pub operation: WireOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WireOperation {
    ExternCall {
        name: String,
        extern_index: u32,
        args: Vec<WireValue>,
    },
    EntitySpawn {
        type_index: u32,
    },
    FieldRead {
        entity: WireHandle,
        field_index: u32,
    },
    FieldWrite {
        entity: WireHandle,
        field_index: u32,
        value: WireValue,
    },
    GetComponent {
        entity: WireHandle,
        component_type_index: u32,
    },
    InitEntity {
        entity: WireHandle,
    },
    DestroyEntity {
        entity: WireHandle,
    },
    GetOrCreate {
        type_index: u32,
    },
    Join {
        target_task: WireHandle,
    },
}

pub fn value_to_wire<H: RuntimeHost>(runtime: &Runtime<H>, value: Value) -> WireValue {
    match value {
        Value::Void => WireValue::Void,
        Value::Int(value) => WireValue::Int {
            value: value.to_string(),
        },
        Value::Float(value) => WireValue::Float { value },
        Value::Bool(value) => WireValue::Bool { value },
        Value::Entity(entity) => WireValue::Entity {
            index: entity.index,
            generation: entity.generation,
        },
        Value::Ref(reference) => runtime
            .heap()
            .read_string(reference)
            .map(|value| WireValue::String {
                value: value.to_owned(),
            })
            .unwrap_or_else(|_| WireValue::Opaque {
                display: "<heap value>".into(),
            }),
        Value::Struct { type_idx, .. } => WireValue::Opaque {
            display: format!("<struct@{type_idx}>"),
        },
    }
}

pub fn value_from_wire<H: RuntimeHost>(
    runtime: &mut Runtime<H>,
    value: WireValue,
) -> Result<Value, String> {
    match value {
        WireValue::Void => Ok(Value::Void),
        WireValue::Int { value } => value
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| format!("invalid signed 64-bit integer '{value}'")),
        WireValue::Float { value } if value.is_finite() => Ok(Value::Float(value)),
        WireValue::Float { .. } => Err("float must be finite".into()),
        WireValue::Bool { value } => Ok(Value::Bool(value)),
        WireValue::String { value } => Ok(Value::Ref(runtime.heap_mut().alloc_string(&value))),
        WireValue::Entity { index, generation } => {
            Ok(Value::Entity(EntityId::new(index, generation)))
        }
        WireValue::Opaque { .. } => Err("opaque heap values cannot be passed into the VM".into()),
    }
}

pub fn response_from_wire<H: RuntimeHost>(
    runtime: &mut Runtime<H>,
    response: WireResponse,
) -> Result<HostResponse, String> {
    match response {
        WireResponse::Value { value } => value_from_wire(runtime, value).map(HostResponse::Value),
        WireResponse::Entity { index, generation } => {
            Ok(HostResponse::EntityHandle(EntityId::new(index, generation)))
        }
        WireResponse::Confirmed => Ok(HostResponse::Confirmed),
        WireResponse::Deferred => Ok(HostResponse::Suspend),
        WireResponse::Error { message } => Ok(HostResponse::Error(HostError::Failed(message))),
    }
}

pub fn request_to_wire<H: RuntimeHost>(
    runtime: &Runtime<H>,
    id: u32,
    request: &HostRequest,
    extern_names: &[String],
) -> WireRequest {
    let (task_id, operation) = match request {
        HostRequest::ExternCall {
            task_id,
            extern_idx,
            args,
            ..
        } => (
            *task_id,
            WireOperation::ExternCall {
                name: extern_names
                    .get(((*extern_idx & 0x00ff_ffff) as usize).saturating_sub(1))
                    .cloned()
                    .unwrap_or_else(|| format!("extern#{extern_idx}")),
                extern_index: (((*extern_idx & 0x00ff_ffff) as usize).saturating_sub(1)) as u32,
                args: args
                    .iter()
                    .copied()
                    .map(|v| value_to_wire(runtime, v))
                    .collect(),
            },
        ),
        HostRequest::EntitySpawn { task_id, type_idx } => (
            *task_id,
            WireOperation::EntitySpawn {
                type_index: *type_idx,
            },
        ),
        HostRequest::FieldRead {
            task_id,
            entity,
            field_idx,
        } => (
            *task_id,
            WireOperation::FieldRead {
                entity: handle(*entity),
                field_index: *field_idx,
            },
        ),
        HostRequest::FieldWrite {
            task_id,
            entity,
            field_idx,
            value,
        } => (
            *task_id,
            WireOperation::FieldWrite {
                entity: handle(*entity),
                field_index: *field_idx,
                value: value_to_wire(runtime, *value),
            },
        ),
        HostRequest::GetComponent {
            task_id,
            entity,
            comp_type_idx,
        } => (
            *task_id,
            WireOperation::GetComponent {
                entity: handle(*entity),
                component_type_index: *comp_type_idx,
            },
        ),
        HostRequest::InitEntity { task_id, entity } => (
            *task_id,
            WireOperation::InitEntity {
                entity: handle(*entity),
            },
        ),
        HostRequest::DestroyEntity { task_id, entity } => (
            *task_id,
            WireOperation::DestroyEntity {
                entity: handle(*entity),
            },
        ),
        HostRequest::GetOrCreate { task_id, type_idx } => (
            *task_id,
            WireOperation::GetOrCreate {
                type_index: *type_idx,
            },
        ),
        HostRequest::Join {
            task_id,
            target_task,
        } => (
            *task_id,
            WireOperation::Join {
                target_task: handle(*target_task),
            },
        ),
    };
    WireRequest {
        id,
        task: handle(task_id),
        operation,
    }
}

fn handle<T>(value: writ_runtime::GenHandle<T>) -> WireHandle {
    WireHandle {
        index: value.index,
        generation: value.generation,
    }
}
