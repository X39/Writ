# Technology Stack

**Analysis Date:** 2026-02-26

## Languages

**Primary:**
- Rust 2024 edition - Parser, compiler, and runtime implementation for the Writ scripting language

**Secondary:**
- None currently in use

## Runtime

**Environment:**
- Rust compiler toolchain (stable track inferred from Cargo.toml edition "2024")
- Cross-platform: Linux, macOS, Windows (via rustix and windows-sys support)

**Package Manager:**
- Cargo - Official Rust package manager
- Lockfile: Present (`Cargo.lock` - automatically generated)

## Frameworks

**Core:**
- Chumsky 0.12.0 (with `pratt` feature) - Expression parser combinator library
  - Handles operator precedence (13+ levels per spec)
  - Provides recursive descent parsing with mutual recursion support
  - Used in `/d/dev/git/Writ/writ-parser/src/parser.rs`
- Logos 0.16.1 - Lexical scanner/tokenizer with regex-based rules
  - Derives token definitions via macros
  - Used in `/d/dev/git/Writ/writ-parser/src/lexer.rs`

**Testing:**
- Insta 1.46.3 (with `ron` feature) - Snapshot testing framework
  - RON (Rusty Object Notation) format for snapshots
  - Test files in `/d/dev/git/Writ/writ-parser/tests/`
- Integration with Rust's built-in `#[test]` attribute

**Build/Dev:**
- Cargo (workspace manifest at `/d/dev/git/Writ/Cargo.toml`)
- Ariadne 0.6.0 (dev dependency) - Pretty error reporting with source spans
  - Used for test error reporting only

## Key Dependencies

**Critical:**
- Chumsky 0.12.0 - Parser combinators for recursive descent parsing with error recovery
- Logos 0.16.1 - High-performance lexical analysis with full-fidelity trivia preservation

**Serialization & Data:**
- Serde 1.0.228 - Serialization framework (used transitively)
- Serde_json 1.0.149 - JSON serialization (used by wit-component)
- RON 0.12.0 - Rusty Object Notation for test snapshots

**String & Unicode:**
- Unicode-segmentation 1.12.0 - Unicode text segmentation
- Unicode-ident 1.0.24 - Unicode identifier support
- Unicode-width 0.2.2 - Character width calculation for display

**Platform Support:**
- Windows-sys 0.59.0 - Windows API bindings
- Rustix 1.1.4 - Rust interface to Unix-like system calls
- Libc 0.2.182 - C standard library bindings

**WebAssembly (transitive):**
- Wit-bindgen 0.51.0 - WebAssembly Interface Type code generator
- Wasm-encoder 0.244.0 - WebAssembly binary encoder
- Wasm-metadata 0.244.0 - WebAssembly metadata manipulation
- Wasmparser 0.244.0 - WebAssembly binary parser

**Other:**
- Anyhow 1.0.102 - Ergonomic error handling (transitive)
- Tempfile 3.26.0 - Temporary file/directory creation (used by insta)
- Similar 2.7.0 - Diff computation (used by insta)
- Log 0.4.29 - Logging facade (used by wit-component)

## Workspace Structure

The project is organized as a Rust workspace with four crates:

```
/d/dev/git/Writ/
├── writ-parser/      [0.1.0] - Language lexer, parser, and CST (Concrete Syntax Tree)
├── writ-cli/         [0.1.0] - Command-line interface (stub)
├── writ-compiler/    [0.1.0] - Compiler backend (stub)
└── writ-runtime/     [0.1.0] - Runtime environment (stub)
```

## Configuration

**Environment:**
- No `.env` files or environment-based configuration detected
- Configuration via Cargo.toml workspace and per-crate manifests
- Default Rust compiler settings (2024 edition implies latest language features)

**Build:**
- `Cargo.toml` - Workspace root manifest
- Per-crate manifests: `writ-parser/Cargo.toml`, `writ-cli/Cargo.toml`, `writ-compiler/Cargo.toml`, `writ-runtime/Cargo.toml`
- No custom build scripts (`build.rs` files) detected
- Cargo.lock ensures reproducible builds

## Platform Requirements

**Development:**
- Rust 2024 edition or later
- Cargo package manager
- Supported on: Linux, macOS, Windows
- No OS-specific dependencies beyond rustix/windows-sys

**Production:**
- Standalone Rust binaries
- Self-contained deployment (no external runtime required beyond system libraries)
- Single executable for CLI

## Build Process

```bash
# Build all crates
cargo build --verbose

# Run tests
cargo test --verbose
```

Workspace resolver version 3 supports parallel builds and shared dependencies.

---

*Stack analysis: 2026-02-26*
