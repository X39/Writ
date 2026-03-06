//! Synthetic ExternDef injection for log-level and dialogue builtin functions.

use rustc_hash::FxHashSet;

use crate::resolve::def_map::{DefId, DefMap};

use crate::emit::module_builder::ModuleBuilder;

// =============================================================================
// Synthetic log-level ExternDef injection
// =============================================================================

/// Inject ExternDef rows for synthetic log-level builtin functions that are
/// actually referenced by the source code.
///
/// These have no AST entry (they are injected by inject_log_namespace in the resolver),
/// so we encode the sig blob directly: (string) -> void.
///
/// Encoding per spec §2.15.3:
///   u16 param_count (LE) + param TypeRef bytes + return TypeRef byte
///   string = 0x04, void = 0x00
///   Result: [0x01, 0x00, 0x04, 0x00]
///
/// Must be called AFTER all user-declared externs have been collected so that
/// existing extern token indices are not shifted.
pub fn inject_log_extern_defs(
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
    called_ids: &FxHashSet<DefId>,
) {
    // Sig blob for (string) -> void: param_count=1 (u16 LE), string type tag, void return tag.
    let sig_bytes: Vec<u8> = vec![0x01, 0x00, 0x04, 0x00];
    let sig_blob = builder.blob_heap.intern(&sig_bytes);

    for &level in crate::resolve::prelude::LOG_NAMESPACE_LEVELS {
        let fqn = format!("log::{}", level);
        if let Some(def_id) = def_map.get(&fqn) {
            // Only emit the ExternDef row if the source actually calls this log function.
            if !called_ids.contains(&def_id) {
                continue;
            }
            builder.add_extern_def(
                &fqn,     // name in ExternDef table (e.g. "log::info")
                sig_blob,
                &fqn,     // import_name = same as name
                1,        // flags: pub
                Some(def_id),
            );
        }
    }
}

// =============================================================================
// Synthetic dialogue-builtin ExternDef injection
// =============================================================================

/// Inject ExternDef rows for synthetic dialogue builtin functions that are
/// actually referenced by the source code.
///
/// These have no AST entry (they are injected by inject_dialogue_namespace in the resolver).
/// Each builtin gets its own sig blob encoding.
///
/// Must be called AFTER all user-declared externs have been collected.
pub fn inject_dialogue_extern_defs(
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
    called_ids: &FxHashSet<DefId>,
) {
    // Pre-intern the fn() -> void blob for ChoiceOption's third param.
    // fn() -> void sig blob: param_count=0 (u16 LE), void return tag.
    let fn_void_blob_bytes: Vec<u8> = vec![0x00, 0x00, 0x00];
    let fn_void_blob_offset = builder.blob_heap.intern(&fn_void_blob_bytes);

    // Build sig blobs for each dialogue builtin.
    let builtins: &[(&str, Vec<u8>)] = &[
        // say(text: string) -> void: 1 param, string, void return
        ("say", vec![0x01, 0x00, 0x04, 0x00]),
        // say_localized(key: string, locale: string) -> void: 2 params, string, string, void return
        ("say_localized", vec![0x02, 0x00, 0x04, 0x04, 0x00]),
        // choice(options: Array<int>) -> int: 1 param, Array<int>, int return
        ("choice", vec![0x01, 0x00, 0x20, 0x01, 0x01]),
        // ChoiceOption(label: string, key: string, body: fn() -> void) -> int:
        // 3 params, string, string, func(blob_offset), int return
        ("ChoiceOption", {
            let mut blob = vec![0x03, 0x00, 0x04, 0x04, 0x30];
            blob.extend_from_slice(&fn_void_blob_offset.to_le_bytes());
            blob.push(0x01); // int return
            blob
        }),
    ];

    for (name, sig_bytes) in builtins {
        if let Some(def_id) = def_map.get(name) {
            if !called_ids.contains(&def_id) {
                continue;
            }
            let sig_blob = builder.blob_heap.intern(sig_bytes);
            builder.add_extern_def(
                name,
                sig_blob,
                name,
                1, // flags: pub
                Some(def_id),
            );
        }
    }
}
