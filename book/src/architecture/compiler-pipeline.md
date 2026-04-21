# Compiler Pipeline

The Writ compiler transforms `.writ` source files into `.writc` binary modules through a 5-stage
pipeline. Each stage has a well-defined input and output type, and each stage is implemented in a
dedicated crate or module.

## Pipeline Stages

| Stage | Name | Input | Output | Crate |
|-------|------|-------|--------|-------|
| 1 | Parse | `.writ` source | CST (concrete syntax tree) | `writ-parser` |
| 2 | Lower | CST | AST (abstract syntax tree) | `writ-compiler` (lower) |
| 3 | Resolve | AST | Resolved AST (names bound) | `writ-compiler` (resolve) |
| 4 | Typecheck | Resolved AST | Typed AST (types verified) | `writ-compiler` (check) |
| 5 | Codegen | Typed AST | `.writc` binary module | `writ-compiler` (emit) |

## Stage Descriptions

**Parse** — Tokenizes the source file using a logos-based lexer and builds a lossless concrete
syntax tree using chumsky parser combinators. The CST preserves all tokens, whitespace, and
punctuation needed for error reporting and future tooling. Lives in the `writ-parser` crate.

**Lower** — Transforms the CST into an abstract syntax tree, desugaring constructs like format
strings (`$"Hello {name}!"`), dialogue blocks, compound assignment operators, and entity
declarations into canonical forms. Only the information needed by semantic analysis is retained.
Lives in the `writ-compiler` `lower` module.

**Resolve** — Performs name resolution, binding every identifier to its definition. Builds a
definition map, resolves imports and `use` declarations, expands qualified paths, and injects
built-in prelude symbols. Emits diagnostic errors for undefined names and ambiguous references.
Lives in the `writ-compiler` `resolve` module.

**Typecheck** — Verifies type correctness across the entire program. Infers types where possible
using unification-based inference, checks contract conformance, validates function signatures, and
enforces strict mutability rules. Lives in the `writ-compiler` `check` module.

**Codegen** — Emits the binary `.writc` module format. Generates IL instructions, populates the
21 metadata tables (TypeDef, MethodDef, FieldDef, and more), and writes the 200-byte module header.
Lives in the `writ-compiler` `emit` module. (Called "emit" in source, "codegen" in documentation.)

## Pipeline Integration

The `writ-cli` crate orchestrates the full pipeline in `pipeline.rs`, calling each stage in sequence
and collecting diagnostics. `writ compile` runs the full pipeline on a single `.writ` file and
writes the resulting `.writc` to disk. `writ build` runs the pipeline for all source files declared
in a `writ.toml` project manifest, emitting a single linked module.
