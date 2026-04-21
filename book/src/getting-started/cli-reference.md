# CLI Reference

The `writ` command-line tool provides subcommands for creating, building, compiling,
assembling, disassembling, and running Writ programs.

## writ new

Creates a new Writ project directory with the standard layout.

**Usage:**

```bash
writ new <name>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<name>` | Yes | Project name. Alphanumeric characters, hyphens, and underscores only. |

**What it creates:**

```
<name>/
├── writ.toml               # Project configuration
├── .gitignore
├── sources/
│   └── main.writ           # Entry point source file
└── bin/
    └── configuration/
```

**Example:**

```bash
writ new my-game
```

## writ build

Compiles all `.writ` source files in a project directory, reading configuration from `writ.toml`.

**Usage:**

```bash
writ build [path] [OPTIONS]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `[path]` | No | Project directory containing `writ.toml`. Defaults to `.` (current directory). |

**Options:**

| Flag | Description |
|------|-------------|
| `--release` | Compile with release profile (strips debug info). |
| `--debug` | Compile with debug profile (default; includes debug info). |
| `--name <name>` | Override the output module name (default: `project.name` from `writ.toml`). |

Output is written to `build/<profile>/<module_name>.writc`.

**Example:**

```bash
writ build --release
writ build ./my-project --name custom-output
```

## writ compile

Compiles a single `.writ` source file to a binary `.writc` module.

**Usage:**

```bash
writ compile <input> [OPTIONS]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<input>` | Yes | Input `.writ` source file path. |

**Options:**

| Flag | Description |
|------|-------------|
| `-o`, `--output <path>` | Output `.writc` path. Defaults to the input path with `.writ` replaced by `.writc`. |

Single-file compilation always emits debug info. If a directory is passed as input,
`writ compile` rejects it with a message directing you to use `writ build` instead.

**Example:**

```bash
writ compile hello.writ
writ compile hello.writ -o output/hello.writc
```

## writ assemble

Assembles a `.writil` text IL file to a binary `.writc` module.

**Usage:**

```bash
writ assemble <input> [OPTIONS]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<input>` | Yes | Input `.writil` file path, or `-` to read from stdin. |

**Options:**

| Flag | Description |
|------|-------------|
| `-o`, `--output <path>` | Output `.writc` path. Defaults to the input path with `.writil` replaced by `.writc`. |

**Example:**

```bash
writ assemble program.writil
writ assemble - -o out.writc < program.writil
```

## writ disasm

Disassembles a binary `.writc` module to `.writil` text format.

**Usage:**

```bash
writ disasm <input> [OPTIONS]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<input>` | Yes | Input `.writc` binary file. |

**Options:**

| Flag | Description |
|------|-------------|
| `--verbose` | Include hex byte offsets and opcode comments for each instruction. |

Output is written to stdout.

**Example:**

```bash
writ disasm hello.writc
writ disasm hello.writc --verbose
```

## writ run

Executes a compiled `.writc` binary module.

**Usage:**

```bash
writ run <input> [OPTIONS]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<input>` | Yes | Input `.writc` binary module file. |

**Options:**

| Flag | Description |
|------|-------------|
| `--entry <name>` | Name of the exported method to run. Defaults to `main`. |
| `--interactive` | Enable interactive choice prompts. By default, choices auto-select option 0. |
| `--verbose` | Print execution statistics and GC info after the run completes. |

**Example:**

```bash
writ run hello.writc
writ run game.writc --entry start_scene --interactive --verbose
```
