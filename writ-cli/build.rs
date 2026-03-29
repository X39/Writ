//! Build script: compiles writ-std source into a .writc binary for embedding.
//!
//! Runs at `cargo build` time. Reads `../writ-std/src/collections.writ`, compiles
//! it to IL bytes via `writ_compiler::compile_source`, and writes the output to
//! `$OUT_DIR/writ-std.writc` for `include_bytes!` in the CLI commands.
//!
//! Phase 120: If stdlib compilation fails (e.g., because writ-std still uses
//! removed array methods — see Phase 121 for the stdlib rewrite), this build
//! script writes an empty placeholder .writc instead of panicking.
//! The CLI commands will detect the empty placeholder and skip stdlib loading.

fn main() {
    // Re-run if the writ-std source changes
    println!("cargo:rerun-if-changed=../writ-std/src/collections.writ");

    let src = include_str!("../writ-std/src/collections.writ");
    // compile_source requires &'static str — leak the owned string
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());

    // Compile on a 16MB stack thread to handle deep AST recursion
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || writ_compiler::compile_source(src_static))
        .expect("failed to spawn compile thread");

    let result = handle
        .join()
        .expect("compile thread panicked");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir).join("writ-std.writc");

    match result {
        Ok(bytes) => {
            std::fs::write(&out_path, &bytes).expect("failed to write writ-std.writc");
        }
        Err(e) => {
            // Stdlib compilation failed — write empty placeholder so the workspace builds.
            // Phase 121 will fix the stdlib source to use the new array API.
            eprintln!(
                "cargo:warning=writ-std compilation failed (expected during Phase 120 — stdlib uses removed array methods; Phase 121 will fix): {}",
                e
            );
            std::fs::write(&out_path, &[]).expect("failed to write empty writ-std.writc placeholder");
        }
    }
}
