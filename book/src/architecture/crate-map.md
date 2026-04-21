# Crate Map

The Writ workspace contains 10 Rust crates. Three are foundation crates with no workspace
dependencies; the rest build on them in well-defined layers.

## Crates

| Crate | Binary | Purpose | Workspace Dependencies |
|-------|--------|---------|------------------------|
| `writ-diagnostics` | -- | Diagnostic rendering and error codes | (none) |
| `writ-module` | -- | IL binary module format (reader/writer) | (none) |
| `writ-parser` | -- | Lexer and CST parser | (none) |
| `writ-assembler` | -- | Text IL assembler and disassembler | `writ-module` |
| `writ-compiler` | -- | Name resolution, type checking, IL codegen | `writ-parser`, `writ-diagnostics`, `writ-module` |
| `writ-runtime` | -- | Register-based VM, task scheduler, entity system | `writ-module` |
| `writ-lsp` | `writ-lsp` | Language server (LSP) | `writ-compiler`, `writ-diagnostics`, `writ-parser`, `writ-module`, `writ-runtime` |
| `writ-dap` | `writ-dap` | Debug adapter (DAP) | `writ-compiler`, `writ-runtime`, `writ-module`, `writ-diagnostics`, `writ-parser` |
| `writ-golden` | -- | Golden file integration tests | `writ-compiler`, `writ-assembler`, `writ-module`, `writ-diagnostics`, `writ-parser` |
| `writ-cli` | `writ` | Command-line tool | `writ-module`, `writ-runtime`, `writ-assembler`, `writ-compiler`, `writ-diagnostics`, `writ-parser` |

```admonish note
`writ-golden` is a test-only crate — it has no exported binary or library and is used exclusively
for golden file integration tests.
```

## Dependency Layers

The workspace is structured in three layers. The foundation crates (`writ-diagnostics`,
`writ-module`, `writ-parser`) have no workspace dependencies and can evolve independently of each
other. The compiler builds on all three foundations, while the runtime depends only on
`writ-module` — this keeps the VM lean and free of parser or diagnostic concerns. The CLI, LSP,
and DAP sit at the top layer, depending on both compiler and runtime; they are the integration
points where all lower layers converge.

## Contributing

### Building

```bash
cargo build --workspace          # debug build
cargo build --workspace --release  # release build (slower; enables fat LTO)
```

### Testing

```bash
cargo test --workspace           # run all tests across all crates
cargo test -p <crate>            # run tests for a single crate (e.g. -p writ-compiler)
cargo test -p writ-golden        # run golden file integration tests
```

### Pull Requests

Fork the repository and create a branch from `master`. Make your changes, ensure
`cargo test --workspace` passes with no failures, then submit a pull request against `master`.
Each crate has its own test suite; `cargo test -p <crate>` is a fast way to verify an individual
layer during development.
