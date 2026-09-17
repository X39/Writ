//! Canonical core-library injection shared by compiler entry points.

use writ_module::Module;

pub(crate) const CORE_MODULE_NAME: &str = "writ-runtime";

/// Return whether `module` declares itself as the canonical core module.
///
/// Module identity is taken from the first ModuleDef row, which is the
/// authoritative identity stored in the module metadata tables.
pub(crate) fn is_core_module(module: &Module) -> bool {
    module.module_defs.first().and_then(|module_def| {
        writ_module::heap::read_string(&module.string_heap, module_def.name).ok()
    }) == Some(CORE_MODULE_NAME)
}

/// Preserve explicit dependency order while ensuring exactly one core module.
///
/// The first explicitly supplied core module is authoritative. Later core
/// modules are ignored. If no core module was supplied, `canonical` is appended
/// after every explicit dependency so their synthetic FileIds remain stable.
pub(crate) fn normalize_libraries<'a>(
    supplied: &[&'a Module],
    canonical: &'a Module,
) -> Vec<&'a Module> {
    let mut normalized = Vec::with_capacity(supplied.len() + 1);
    let mut found_core = false;

    for &module in supplied {
        if is_core_module(module) {
            if found_core {
                continue;
            }
            found_core = true;
        }
        normalized.push(module);
    }

    if !found_core {
        normalized.push(canonical);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::{ModuleBuilder, TypeDefKind};

    fn module(name: &str) -> Module {
        ModuleBuilder::new(name).build()
    }

    #[test]
    fn appends_canonical_core_after_explicit_libraries() {
        let library = module("library");
        let canonical = writ_module::build_writ_runtime_module();

        let normalized = normalize_libraries(&[&library], &canonical);

        assert!(std::ptr::eq(normalized[0], &library));
        assert!(std::ptr::eq(normalized[1], &canonical));
    }

    #[test]
    fn first_explicit_core_is_authoritative() {
        let mut first_builder = ModuleBuilder::new(CORE_MODULE_NAME);
        first_builder.add_type_def("FirstOnly", "writ", TypeDefKind::Struct, 0);
        let first = first_builder.build();
        let second = writ_module::build_writ_runtime_module();
        let canonical = writ_module::build_writ_runtime_module();

        let normalized = normalize_libraries(&[&first, &second], &canonical);

        assert_eq!(normalized.len(), 1);
        assert!(std::ptr::eq(normalized[0], &first));
    }
}
