# Installation

## Prerequisites

- **Rust 1.85 or later** (required for edition 2024 and workspace resolver v3)
  Install via [rustup](https://rustup.rs/): `rustup update stable`
- **Git** for cloning the repository

## Building from Source

Clone the repository and build the workspace:

```bash
git clone https://github.com/X39/Writ.git
cd Writ
cargo build --workspace
```

For a release build:

```bash
cargo build --workspace --release
```

```admonish note
The release profile uses fat LTO and `codegen-units = 1`. This produces a faster binary
but significantly increases compile time.
```

The `writ` binary is placed at:
- Debug: `target/debug/writ`
- Release: `target/release/writ`

## Verifying the Installation

Run the test suite to confirm everything builds and passes:

```bash
cargo test --workspace
```

Check that the CLI is available:

```bash
./target/debug/writ --help
```

Expected output:

```
Writ IL toolchain

Usage: writ <COMMAND>

Commands:
  new       Create a new Writ project
  build     Compile all .writ sources in a Writ project directory
  compile   Compile a .writ source file to a binary .writc module
  assemble  Assemble a .writil text file to a binary .writc module
  disasm    Disassemble a binary .writc module to .writil text
  run       Run a binary .writc module's entry task
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```admonish tip
Use `writ new <name>` to scaffold a new Writ project with the correct directory structure
(`writ.toml`, `sources/main.writ`, `.gitignore`, and `bin/configuration/`) automatically.
```
