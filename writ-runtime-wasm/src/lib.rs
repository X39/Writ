//! Browser-safe WebAssembly adapter for the Writ VM.
//!
//! The core runtime remains platform-neutral. This crate translates its host protocol into
//! stable, serde-defined JavaScript objects and never exposes heap references or Rust types.

#![cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]

mod wire;

use std::collections::HashSet;
use writ_module::heap::read_string;
use writ_runtime::{
    ExecutionLimit, HostRequest, HostResponse, LogLevel, PendingRequest, RequestId, Runtime,
    RuntimeBuilder, RuntimeHost, TaskId, TaskState, TickResult,
};

pub use wire::{WireHandle, WireOperation, WireRequest, WireResponse, WireValue};

#[derive(Default)]
struct WasmHost {
    logs: Vec<(LogLevel, String)>,
    extern_names: Vec<String>,
}

impl RuntimeHost for WasmHost {
    fn on_request(&mut self, _id: RequestId, request: &HostRequest) -> HostResponse {
        if let HostRequest::ExternCall {
            extern_idx,
            display_args,
            ..
        } = request
        {
            let name = self
                .extern_names
                .get(extern_row(*extern_idx))
                .map(String::as_str)
                .unwrap_or("");
            let level = match name {
                "log::trace" => Some(LogLevel::Trace),
                "log::debug" => Some(LogLevel::Debug),
                "log::info" => Some(LogLevel::Info),
                "log::warn" => Some(LogLevel::Warn),
                "log::error" => Some(LogLevel::Error),
                _ => None,
            };
            if let Some(level) = level {
                self.logs
                    .push((level, display_args.first().cloned().unwrap_or_default()));
                return HostResponse::Confirmed;
            }
        }
        // JS is called after control returns from Runtime::tick. This avoids re-entering the VM
        // and works identically on a browser main thread and in a Worker.
        HostResponse::Suspend
    }

    fn on_log(&mut self, level: LogLevel, message: &str) {
        self.logs.push((level, message.to_owned()));
    }
}

fn extern_row(extern_idx: u32) -> usize {
    ((extern_idx & 0x00ff_ffff) as usize).saturating_sub(1)
}

struct Engine {
    libraries: Vec<writ_module::Module>,
    runtime: Option<Runtime<WasmHost>>,
    extern_names: Vec<String>,
    pending: Vec<PendingRequest>,
    dispatched: HashSet<u32>,
    last_error: Option<String>,
}

impl Engine {
    fn new() -> Self {
        Self {
            libraries: vec![],
            runtime: None,
            extern_names: vec![],
            pending: vec![],
            dispatched: HashSet::new(),
            last_error: None,
        }
    }

    fn add_library(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.runtime.is_some() {
            return Err("libraries must be added before loadModule".into());
        }
        self.libraries.push(
            writ_module::Module::from_bytes(bytes)
                .map_err(|e| format!("invalid library .writc module: {e}"))?,
        );
        Ok(())
    }

    fn load_module(&mut self, bytes: &[u8]) -> Result<(), String> {
        let module = writ_module::Module::from_bytes(bytes)
            .map_err(|e| format!("invalid .writc module: {e}"))?;
        self.extern_names = module
            .extern_defs
            .iter()
            .map(|row| {
                let import = read_string(&module.string_heap, row.import_name).unwrap_or("");
                if import.is_empty() {
                    read_string(&module.string_heap, row.name)
                        .unwrap_or("<unknown>")
                        .to_owned()
                } else {
                    import.to_owned()
                }
            })
            .collect();
        let host = WasmHost {
            logs: vec![],
            extern_names: self.extern_names.clone(),
        };
        let mut builder = RuntimeBuilder::new(module).with_host(host);
        for library in self.libraries.drain(..) {
            builder = builder.with_library(library);
        }
        self.runtime = Some(builder.build().map_err(|e| e.to_string())?);
        self.pending.clear();
        self.dispatched.clear();
        self.last_error = None;
        Ok(())
    }

    fn runtime(&self) -> Result<&Runtime<WasmHost>, String> {
        self.runtime
            .as_ref()
            .ok_or_else(|| "no module loaded; call loadModule first".into())
    }
    fn runtime_mut(&mut self) -> Result<&mut Runtime<WasmHost>, String> {
        self.runtime
            .as_mut()
            .ok_or_else(|| "no module loaded; call loadModule first".into())
    }

    fn spawn(&mut self, entry: &str, args: Vec<WireValue>) -> Result<WireHandle, String> {
        let runtime = self.runtime_mut()?;
        let method = runtime
            .find_method(entry)
            .ok_or_else(|| format!("entry point '{entry}' was not found"))?;
        let values = args
            .into_iter()
            .map(|v| wire::value_from_wire(runtime, v))
            .collect::<Result<Vec<_>, _>>()?;
        let id = runtime
            .spawn_task(method, values)
            .map_err(|e| e.to_string())?;
        Ok(WireHandle {
            index: id.index,
            generation: id.generation,
        })
    }

    fn tick(&mut self, delta_time: f64, budget: u32) -> Result<&'static str, String> {
        if budget == 0 {
            return Err("instructionBudget must be greater than zero".into());
        }
        let result = self
            .runtime_mut()?
            .tick(delta_time, ExecutionLimit::Instructions(u64::from(budget)));
        self.pending = self.runtime()?.pending_requests();
        let status = match result {
            TickResult::AllCompleted => "allCompleted",
            TickResult::TasksSuspended(_) => "suspended",
            TickResult::ExecutionLimitReached => "budgetExhausted",
            TickResult::Empty => "empty",
        };
        Ok(status)
    }

    fn pending_wire(&self) -> Result<Vec<WireRequest>, String> {
        let runtime = self.runtime()?;
        Ok(self
            .pending
            .iter()
            .map(|p| wire::request_to_wire(runtime, p.request_id.0, &p.request, &self.extern_names))
            .collect())
    }

    fn resolve(&mut self, request_id: u32, response: WireResponse) -> Result<(), String> {
        if matches!(response, WireResponse::Deferred) {
            return Err("a deferred request must be resolved with a final response".into());
        }
        let runtime = self.runtime_mut()?;
        let response = wire::response_from_wire(runtime, response)?;
        runtime
            .confirm(RequestId(request_id), response)
            .map_err(|e| e.to_string())?;
        self.pending.retain(|p| p.request_id.0 != request_id);
        self.dispatched.remove(&request_id);
        Ok(())
    }

    fn state(&self, handle: &WireHandle) -> Result<&'static str, String> {
        match self
            .runtime()?
            .task_state(TaskId::new(handle.index, handle.generation))
        {
            Some(TaskState::Ready) => Ok("ready"),
            Some(TaskState::Running) => Ok("running"),
            Some(TaskState::Suspended) => Ok("suspended"),
            Some(TaskState::Completed) => Ok("completed"),
            Some(TaskState::Cancelled) => Ok("cancelled"),
            None => Err("unknown or stale task handle".into()),
        }
    }

    fn return_value(&self, handle: &WireHandle) -> Result<Option<WireValue>, String> {
        let runtime = self.runtime()?;
        let id = TaskId::new(handle.index, handle.generation);
        if runtime.task_state(id).is_none() {
            return Err("unknown or stale task handle".into());
        }
        Ok(runtime
            .return_value(id)
            .map(|value| wire::value_to_wire(runtime, value)))
    }

    fn task_error(&self, handle: &WireHandle) -> Result<Option<String>, String> {
        let runtime = self.runtime()?;
        let id = TaskId::new(handle.index, handle.generation);
        if runtime.task_state(id).is_none() {
            return Err("unknown or stale task handle".into());
        }
        Ok(runtime
            .crash_info(id)
            .map(|error| error.format_stacktrace()))
    }
}

#[cfg(target_arch = "wasm32")]
mod bindings {
    use super::*;
    use js_sys::Function;
    use serde::Serialize;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct WritVm {
        engine: Engine,
        host_callback: Option<Function>,
        log_callback: Option<Function>,
    }

    #[wasm_bindgen]
    impl WritVm {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self {
                engine: Engine::new(),
                host_callback: None,
                log_callback: None,
            }
        }

        #[wasm_bindgen(js_name = addLibrary)]
        pub fn add_library(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
            self.engine.add_library(bytes).map_err(js_error)
        }

        #[wasm_bindgen(js_name = loadModule)]
        pub fn load_module(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
            self.engine.load_module(bytes).map_err(js_error)
        }

        #[wasm_bindgen(js_name = setHostCallback)]
        pub fn set_host_callback(&mut self, callback: Function) {
            self.host_callback = Some(callback);
        }

        #[wasm_bindgen(js_name = setLogCallback)]
        pub fn set_log_callback(&mut self, callback: Function) {
            self.log_callback = Some(callback);
        }

        pub fn spawn(&mut self, entry_point: &str, args: JsValue) -> Result<JsValue, JsValue> {
            let args: Vec<WireValue> = serde_wasm_bindgen::from_value(args)
                .map_err(|e| js_error(format!("invalid arguments: {e}")))?;
            to_js(&self.engine.spawn(entry_point, args).map_err(js_error)?)
        }

        pub fn tick(
            &mut self,
            delta_time: f64,
            instruction_budget: u32,
        ) -> Result<JsValue, JsValue> {
            let mut status = self
                .engine
                .tick(delta_time, instruction_budget)
                .map_err(js_error)?;
            self.flush_logs()?;
            if !self.engine.pending.is_empty() {
                self.dispatch_new_requests()?;
            }
            if self.engine.pending.is_empty() && status == "suspended" {
                status = "ready";
            }
            let pending = self.engine.pending.len();
            to_js(&serde_json::json!({"status": status, "pendingRequests": pending}))
        }

        #[wasm_bindgen(js_name = pendingRequests)]
        pub fn pending_requests(&self) -> Result<JsValue, JsValue> {
            to_js(&self.engine.pending_wire().map_err(js_error)?)
        }

        #[wasm_bindgen(js_name = resolveRequest)]
        pub fn resolve_request(
            &mut self,
            request_id: u32,
            response: JsValue,
        ) -> Result<(), JsValue> {
            let response = serde_wasm_bindgen::from_value(response)
                .map_err(|e| js_error(format!("invalid host response: {e}")))?;
            self.engine.resolve(request_id, response).map_err(js_error)
        }

        #[wasm_bindgen(js_name = taskState)]
        pub fn task_state(&self, task: JsValue) -> Result<String, JsValue> {
            let task = parse_handle(task)?;
            self.engine
                .state(&task)
                .map(str::to_owned)
                .map_err(js_error)
        }

        #[wasm_bindgen(js_name = returnValue)]
        pub fn return_value(&self, task: JsValue) -> Result<JsValue, JsValue> {
            let task = parse_handle(task)?;
            to_js(&self.engine.return_value(&task).map_err(js_error)?)
        }

        #[wasm_bindgen(js_name = taskError)]
        pub fn task_error(&self, task: JsValue) -> Result<JsValue, JsValue> {
            let task = parse_handle(task)?;
            to_js(&self.engine.task_error(&task).map_err(js_error)?)
        }

        #[wasm_bindgen(js_name = lastError)]
        pub fn last_error(&self) -> Option<String> {
            self.engine.last_error.clone()
        }

        fn dispatch_new_requests(&mut self) -> Result<(), JsValue> {
            let Some(callback) = self.host_callback.clone() else {
                return Ok(());
            };
            let requests = self.engine.pending_wire().map_err(js_error)?;
            for request in requests {
                if !self.engine.dispatched.insert(request.id) {
                    continue;
                }
                let arg = to_js(&request)?;
                let returned = match callback.call1(&JsValue::UNDEFINED, &arg) {
                    Ok(value) => value,
                    Err(error) => {
                        let message = error
                            .as_string()
                            .unwrap_or_else(|| "host callback threw an exception".into());
                        to_js(&WireResponse::Error { message })?
                    }
                };
                let response: WireResponse =
                    serde_wasm_bindgen::from_value(returned).map_err(|e| {
                        js_error(format!("host callback returned an invalid response: {e}"))
                    })?;
                if !matches!(response, WireResponse::Deferred) {
                    if let Err(error) = self.engine.resolve(request.id, response) {
                        self.engine.last_error = Some(error.clone());
                        return Err(js_error(error));
                    }
                }
            }
            Ok(())
        }

        fn flush_logs(&mut self) -> Result<(), JsValue> {
            let logs =
                std::mem::take(&mut self.engine.runtime_mut().map_err(js_error)?.host_mut().logs);
            if let Some(callback) = &self.log_callback {
                for (level, message) in logs {
                    callback.call2(
                        &JsValue::UNDEFINED,
                        &JsValue::from_str(level_name(level)),
                        &JsValue::from_str(&message),
                    )?;
                }
            }
            Ok(())
        }
    }

    fn parse_handle(value: JsValue) -> Result<WireHandle, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map_err(|e| js_error(format!("invalid task handle: {e}")))
    }
    fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
        value
            .serialize(&serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true))
            .map_err(|e| js_error(e.to_string()))
    }
    fn js_error(message: impl ToString) -> JsValue {
        js_sys::Error::new(&message.to_string()).into()
    }
    fn level_name(level: LogLevel) -> &'static str {
        match level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use bindings::WritVm;

#[cfg(test)]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
