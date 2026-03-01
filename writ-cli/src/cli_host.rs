/// CliHost: RuntimeHost implementation for developer-facing CLI use.
///
/// Annotates all host interactions with type-tagged prefixes:
/// - [say] for extern "say" calls
/// - [choice] for extern "choice" calls (auto-selects 0 in non-interactive mode)
/// - [entity:spawn] for entity spawn requests
/// - [entity:destroy] for entity destroy requests
/// - [extern] for all other extern calls
///
/// # String Arguments in say()
///
/// FIX-03: String arguments are now correctly displayed via `HostRequest::ExternCall::display_args`.
/// The runtime pre-resolves `Value::Ref` args through the GC heap before issuing the request,
/// so `display_args` always contains actual string content. CliHost uses `display_args` for
/// say() output instead of `format_value()`, which produced `<string>` placeholders for Ref args.
use writ_module::{heap::read_string, Module};
use writ_runtime::{GcStats, HostRequest, HostResponse, LogLevel, RequestId, RuntimeHost, Value};

/// A RuntimeHost implementation for CLI use with annotated output.
pub struct CliHost {
    /// Extern function names from the module's ExternDef table (0-indexed, parallel to extern_defs).
    extern_names: Vec<String>,
    /// Whether to prompt interactively for choices.
    interactive: bool,
    /// Whether to emit verbose diagnostics.
    verbose: bool,
    /// Count of on_request calls (approximate request volume tracking).
    request_count: u64,
}

impl CliHost {
    /// Create a CliHost by extracting extern names from the module.
    ///
    /// The `module` reference is used only during construction to build the extern name table.
    pub fn new(module: &Module, interactive: bool, verbose: bool) -> Self {
        let extern_names = module
            .extern_defs
            .iter()
            .map(|ed| {
                read_string(&module.string_heap, ed.name)
                    .unwrap_or("?")
                    .to_string()
            })
            .collect();

        CliHost {
            extern_names,
            interactive,
            verbose,
            request_count: 0,
        }
    }

    /// Format a Value for display in say() output.
    ///
    /// Int, Float, Bool print their values directly.
    /// Ref prints a placeholder — see module-level doc comment for the known limitation.
    fn format_value(v: &Value) -> String {
        match v {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            // Cannot dereference heap ref from host — print placeholder
            Value::Ref(_) => "<string>".to_string(),
            Value::Void => "void".to_string(),
            Value::Entity(e) => format!("<entity@{}>", e.index),
        }
    }

    /// Resolve an extern_idx MetadataToken to a function name.
    ///
    /// The token layout is: bits 31-24 = table_id (16 for ExternDef), bits 23-0 = 1-based row.
    fn resolve_extern_name(&self, extern_idx: u32) -> &str {
        let row_1based = (extern_idx & 0x00FF_FFFF) as usize;
        if row_1based == 0 {
            return "?";
        }
        let idx = row_1based - 1; // convert to 0-based
        self.extern_names.get(idx).map(|s| s.as_str()).unwrap_or("?")
    }

    /// Print execution statistics to stderr (only when verbose).
    pub fn print_stats(&self) {
        if self.verbose {
            eprintln!("[stats] requests={}", self.request_count);
        }
    }
}

impl RuntimeHost for CliHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        self.request_count += 1;

        match req {
            HostRequest::ExternCall { extern_idx, args, display_args, .. } => {
                let name = self.resolve_extern_name(*extern_idx);

                match name {
                    "say" => {
                        // FIX-03: Use display_args for string content; fall back to format_value
                        // for backward compat when display_args is not populated.
                        let text = if !display_args.is_empty() {
                            display_args[0].clone()
                        } else {
                            args.first().map(Self::format_value).unwrap_or_default()
                        };
                        println!("[say] {text}");
                        HostResponse::Value(Value::Void)
                    }
                    "choice" => {
                        // choice() auto-selects 0 in non-interactive mode
                        if self.interactive {
                            // Interactive: prompt on stdin and parse index
                            eprint!("[choice] Enter selection: ");
                            let mut line = String::new();
                            if std::io::stdin().read_line(&mut line).is_ok() {
                                let idx: i64 =
                                    line.trim().parse().unwrap_or(0);
                                HostResponse::Value(Value::Int(idx))
                            } else {
                                println!("[choice] auto-selecting 0 (stdin read failed)");
                                HostResponse::Value(Value::Int(0))
                            }
                        } else {
                            println!("[choice] auto-selecting 0");
                            HostResponse::Value(Value::Int(0))
                        }
                    }
                    other => {
                        println!("[extern] {other}()");
                        HostResponse::Value(Value::Void)
                    }
                }
            }

            HostRequest::EntitySpawn { type_idx, .. } => {
                println!("[entity:spawn] type={type_idx}");
                HostResponse::Confirmed
            }

            HostRequest::DestroyEntity { entity, .. } => {
                println!("[entity:destroy] entity={}:{}", entity.index, entity.generation);
                HostResponse::Confirmed
            }

            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, level: LogLevel, message: &str) {
        eprintln!("[{level:?}] {message}");
    }

    fn on_gc_complete(&mut self, stats: &GcStats) {
        if self.verbose {
            eprintln!(
                "[gc] freed={} traced={} heap_before={} heap_after={}",
                stats.objects_freed,
                stats.objects_traced,
                stats.heap_before,
                stats.heap_after,
            );
        }
    }
}

// ─── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::{heap::intern_string, Module};
    use writ_runtime::{HostRequest, RequestId, TaskId, Value};

    /// Build a minimal Module containing one extern def named `name`.
    fn module_with_extern(name: &str) -> Module {
        let mut m = Module::new();
        // Intern the extern name into the string heap
        let name_offset = intern_string(&mut m.string_heap, name);
        m.extern_defs.push(writ_module::tables::ExternDefRow {
            name: name_offset,
            signature: 0,
            import_name: 0,
            flags: 0,
        });
        m
    }

    #[test]
    fn test_cli_host_extern_call_say_returns_void() {
        let module = module_with_extern("say");
        let mut host = CliHost::new(&module, false, false);
        // ExternDef table_id=16, row=1 => token 0x10000001
        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![Value::Int(42)],
            display_args: vec!["42".to_string()],
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Void) => {}
            other => panic!("expected Value(Void), got {:?}", other),
        }
    }

    #[test]
    fn test_cli_host_choice_auto_returns_zero() {
        let module = module_with_extern("choice");
        let mut host = CliHost::new(&module, false, false);
        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Int(0)) => {}
            other => panic!("expected Value(Int(0)), got {:?}", other),
        }
    }

    #[test]
    fn test_cli_host_entity_spawn_returns_confirmed() {
        let module = Module::new();
        let mut host = CliHost::new(&module, false, false);
        let req = HostRequest::EntitySpawn {
            task_id: TaskId::new(0, 0),
            type_idx: 5,
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Confirmed => {}
            other => panic!("expected Confirmed, got {:?}", other),
        }
    }

    #[test]
    fn test_cli_host_unknown_extern_returns_void() {
        let module = module_with_extern("some_extern");
        let mut host = CliHost::new(&module, false, false);
        let extern_tok: u32 = (16u32 << 24) | 1;
        let req = HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: extern_tok,
            args: vec![],
            display_args: vec![],
        };
        match host.on_request(RequestId(0), &req) {
            HostResponse::Value(Value::Void) => {}
            other => panic!("expected Value(Void), got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_extern_name_first_entry() {
        let module = module_with_extern("greet");
        let host = CliHost::new(&module, false, false);
        let extern_tok: u32 = (16u32 << 24) | 1;
        assert_eq!(host.resolve_extern_name(extern_tok), "greet");
    }

    #[test]
    fn test_resolve_extern_name_out_of_range() {
        let module = Module::new(); // no extern defs
        let host = CliHost::new(&module, false, false);
        let extern_tok: u32 = (16u32 << 24) | 99;
        assert_eq!(host.resolve_extern_name(extern_tok), "?");
    }
}
