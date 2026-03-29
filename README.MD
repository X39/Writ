<p align="center">
    <img src="resources/icon_128.svg" alt="Writ Logo" width="128" height="128">
</p>

# Writ Scripting Language

A game scripting language with first-class dialogue support, C-style syntax, a Rust-inspired type system, and an entity-component architecture.

## Overview

Writ is designed specifically for game development, combining familiar scripting syntax with powerful features for interactive storytelling and gameplay logic. It provides native constructs for branching conversations, speaker management, and localization — making dialogue a first-class language construct rather than an afterthought.

The type system draws heavily from Rust: strong static typing, `Option<T>`/`Result<T,E>` for nullable and fallible values, pattern matching, explicit mutability, and a trait-like contract system for polymorphism. Entity declarations and lifecycle hooks are built directly into the language, with an extern-only component model that keeps host-managed data separate from script logic.

The current implementation is a production-quality reference toolchain: a complete compiler pipeline (frontend, type checker, IL code generator), a full IL runtime, a language server, a debug adapter, and a VS Code extension. 79,005 LOC Rust across 10 crates.

## Features

### First-class Dialogue Support

Native `dlg` constructs handle branching conversations, speaker resolution across three scopes, choice scoping, and automatic localization key generation (FNV-1a auto-keys with `#key` overrides and collision detection).

### Rust-inspired Type System

Strong static typing with `Option<T>`/`Result<T,E>`, pattern matching, and explicit `mut` annotations. Operator overloading compiles to contract implementations. Strict binding mutability: `let` prevents both reassignment and mutation.

### Entity-Component Architecture

Built-in `entity` declarations with component slots (extern-only, data-only — no script-defined components). Universal lifecycle hooks (`on create`, `on finalize`, `on serialize`, `on deserialize`) and entity-specific hooks (`on destroy`, `on interact`).

### Register-based IL Runtime

A 91-instruction register-based VM with a 5-state cooperative task scheduler, mark-sweep GC with finalization queues, contract dispatch via CALL_VIRT, and a suspend-and-confirm host interface at 9 transition points. Entity handles are generation-indexed for stale-handle detection.

### Developer Tooling

Full editor integration via the `writ-lsp` language server and `writ-dap` debug adapter, packaged in the `writ-vscode/` VS Code extension:

- **LSP:** diagnostics, completions (identifiers, keywords, fields, namespaces, post-`new` type suggestions), hover, go-to-definition, find references, signature help, semantic token highlighting
- **DAP:** breakpoints, step over/into/out, call stack inspection, variable and watch expression evaluation, crash halt with break-before-unwind and live variable inspection
- **Crash stacktraces:** full stack trace with source locations displayed in the editor when the runtime crashes

### Benchmark Suite

Cross-language benchmarks in `benchmark/` comparing Writ against Lua, Python, Ruby, Perl, and Node.js across 7 case categories (fibonacci, sieve, string concat, array sort, hash map, OOP dispatch, object creation). Reproducible Docker-based runner with SVG chart generation and GitHub Actions CI workflow.

## Project Structure

```
writ-parser       -- Lexer (logos) and CST parser (chumsky)
writ-compiler     -- Name resolution, type checking, and IL code generation
writ-module       -- IL binary module format (reader/writer, 200-byte header, 21 metadata tables)
writ-runtime      -- Register-based VM, task scheduler, entity system, GC, contract dispatch
writ-assembler    -- Text IL assembler and disassembler
writ-diagnostics  -- Diagnostic rendering and error codes
writ-lsp          -- Language server (LSP) for editor intelligence
writ-dap          -- Debug adapter (DAP) for source-level debugging
writ-golden       -- Golden file integration tests
writ-cli          -- `writ` command-line tool (new, build, compile, assemble, disassemble, run)
writ-vscode/      -- VS Code extension with TextMate grammar, LSP client, DAP client
benchmark/        -- Cross-language benchmark suite (Docker, Python charts, CI workflow)
tools/            -- ImHex pattern for .writc inspection, IL hex viewer
```

## Building

```
cargo build --workspace
cargo test --workspace
```

## CLI

```
writ new <name>         Create a new Writ project
writ build [path]       Build a writ.toml project (--release/--debug)
writ compile <file>     Compile a .writ source file to a .writc binary module
writ assemble <file>    Assemble a .writil text file to a .writc binary module
writ disasm <file>      Disassemble a .writc binary module to .writil text
writ run <file>         Execute a .writc binary module
```

## License

LGPL-3.0-only

## Contributing

Contributions are welcome!
Please submit a pull request or create a discussion to discuss any changes you wish to make.

### Code of Conduct

Be excellent to each other.

### Contributor License Agreement

By submitting a contribution (pull request, patch, or any other form) to this project, you agree
to the following terms:

1. **License Grant.** You grant the project maintainer ("Maintainer") and all recipients of the
   software a perpetual, worldwide, non-exclusive, royalty-free, irrevocable license to use,
   reproduce, modify, distribute, sublicense, and otherwise exploit your contribution under the
   terms of the GNU Lesser General Public License v3.0 (LGPL-3.0-only). You additionally grant
   the Maintainer the right to relicense your contribution under any other open-source or
   proprietary license at the Maintainer's sole discretion.

2. **Originality.** You represent that your contribution is your original work, or that you have
   sufficient rights to grant the licenses above. If your contribution includes third-party
   material, you represent that its license is compatible with the LGPL-3.0-only and permits the
   grants made herein.

3. **No Conflicting Obligations.** You represent that your contribution is not subject to any
   agreement, obligation, or encumbrance (including but not limited to employment agreements or
   prior license grants) that would conflict with or restrict the rights granted above.

4. **No Compensation.** Your contribution is made voluntarily and without expectation of
   compensation, unless separately agreed in writing.

5. **Right to Remove.** The Maintainer may remove, modify, or replace your contribution at any
   time, for any reason, without notice or obligation to you.

6. **Liability.** To the maximum extent permitted by applicable law, your contribution is provided
   "as is", without warranty of any kind. You shall be solely liable for any damage arising from
   the inclusion of your contribution to the extent such damage is caused by a defect, rights
   violation, or other issue originating in your contribution.

7. **Governing Law.** This agreement is governed by the laws of the Federal Republic of Germany
   (Bundesrepublik Deutschland), in particular the German Civil Code (BGB), without regard to
   its conflict-of-laws provisions. For contributors outside Germany, this choice of law applies
   to the extent permitted by the contributor's local jurisdiction.

Please add yourself to the [CONTRIBUTORS](CONTRIBUTORS.md) file when submitting your first pull
request, and include the following statement in your pull request description:

> I have read and agree to the Contributor License Agreement in this project's README.
