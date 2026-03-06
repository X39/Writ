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

pub mod value;
pub mod heap;
pub mod gc;
pub mod entity;
pub mod frame;
pub mod task;
pub mod host;
pub mod error;
pub mod domain;
mod domain_dispatch;
pub mod loader;
pub(crate) mod dispatch;
pub mod scheduler;
pub mod runtime;
pub mod virtual_module;

pub use value::{Value, HeapRef, GenHandle, TaskId, EntityId};
pub use heap::BumpHeap;
pub use gc::{GcHeap, GcStats, GcMode};
pub use entity::{EntityRegistry, EntitySlot, EntityState, PendingEntity};
pub use frame::CallFrame;
pub use task::{Task, TaskState, SuspendReason};
pub use host::{RuntimeHost, HostRequest, HostResponse, NullHost, RequestId, LogLevel, DebugAction};
pub use error::{RuntimeError, CrashInfo, StackFrame, HostError};
pub use domain::{Domain, ResolvedRefs, ResolvedType, ResolvedMethod, ResolvedField};
pub use loader::LoadedModule;
pub use runtime::{Runtime, RuntimeBuilder, ExecutionLimit, TickResult, PendingRequest};
