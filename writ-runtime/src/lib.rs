//! Writ runtime: register-based virtual machine for executing compiled Writ IL.
//!
//! ## Module structure
//!
//! - `value`          -- Value type, HeapRef, GenHandle, TaskId, EntityId
//! - `heap`           -- Bump-allocated heap for value storage
//! - `gc`             -- Mark-sweep garbage collector
//! - `entity`         -- Entity registry and component storage
//! - `frame`          -- Call frame stack management
//! - `task`           -- Task state machine and cooperative yielding
//! - `host`           -- RuntimeHost trait and host request/response protocol
//! - `error`          -- RuntimeError, CrashInfo, HostError types
//! - `domain`         -- Type/method/field resolution from loaded modules
//! - `loader`         -- Module loading and validation
//! - `dispatch`       -- Instruction dispatch loop
//! - `scheduler`      -- Multi-task round-robin scheduler
//! - `runtime`        -- Top-level Runtime API and RuntimeBuilder
//! - `virtual_module` -- writ-runtime built-in module (Option, Result, Range, contracts)
//! - `extern_registry`-- ExternRegistry builder for game-engine extern dispatch

pub(crate) mod dispatch;
pub mod domain;
mod domain_dispatch;
pub mod entity;
pub mod error;
pub mod extern_registry;
pub mod frame;
pub mod gc;
pub mod heap;
pub mod host;
pub mod loader;
pub mod reflection;
pub mod runtime;
pub mod scheduler;
pub mod task;
mod type_specs;
pub mod value;
pub mod virtual_module;

pub use domain::{
    Domain, DomainAttributeMatch, ResolvedField, ResolvedMethod, ResolvedRefs, ResolvedType,
};
pub use entity::{EntityRegistry, EntitySlot, EntityState, EntityTypeIdentity};
pub use error::{CrashInfo, HostError, RuntimeError, StackFrame};
pub use extern_registry::{DeferredCall, ExternHandler, ExternHost, ExternRegistry};
pub use frame::{CallFrame, FrameLocation, RegisterPool};
pub use gc::{GcHeap, GcMode, GcStats};
pub use heap::BumpHeap;
pub use host::{
    AttributeMatch, DebugAction, HostRequest, HostResponse, LogLevel, ModuleAttributeView,
    NullHost, RequestId, RuntimeHost,
};
pub use loader::LoadedModule;
pub use reflection::ReflectionIndex;
pub use runtime::{ExecutionLimit, PendingRequest, Runtime, RuntimeBuilder, TickResult};
pub use task::{SuspendReason, Task, TaskState};
pub use value::{EntityId, GenHandle, HeapRef, TaskId, Value};
