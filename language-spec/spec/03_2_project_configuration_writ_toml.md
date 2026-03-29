# 1.2 Project Configuration (writ.toml)

Every Writ project requires a `writ.toml` file at the project root. This file defines project metadata, compiler
settings, and localization configuration.

## 1.2.1 Format

The file uses [TOML v1.0](https://toml.io/en/v1.0.0) syntax.

## 1.2.2 Required Fields

```toml
[project]
name = "my-game"
version = "0.1.0"

[locale]
default = "en"
```

## 1.2.3 Optional Fields

```toml
[project]
name = "my-game"
version = "0.1.0"
authors = ["Dev Name"]

[locale]
default = "en"
supported = ["en", "de", "fr", "ja", "ko", "zh"]

[compiler]
# Source directories (relative to writ.toml)
sources = ["src/", "dialogue/"]
# Output directory for compiled artifacts
output = "build/"

[profile.debug]
# Include debug info (DebugLocal entries) in compiled module. Default: true
debug_info = true

[profile.release]
# Strip debug info from compiled module. Default: false
debug_info = false

[locale.export]
# Output directory for localization CSV files
output = "locale/"

# Optional: library name mappings for [Import] attributes.
# Maps logical library names to architecture-specific names.
# These serve as defaults — [Import] attribute overrides take precedence.
[libraries.physics]
default = "libphysics"
x64 = "physics64"
arm64 = "physics_arm"

[libraries.audio]
default = "fmod"

# Optional: named conditions for conditional compilation.
# Can be overridden via compiler flags.
[conditions]
debug = true
```

## 1.2.4 Library Resolution

The optional `[libraries.<name>]` sections map logical library names (as used in `[Import("name")]` attributes) to
architecture-specific library names. Each entry supports a `default` key and architecture-specific overrides using the
identifiers defined in [Section 1.25.3](#1253-architecture-identifiers).

Resolution precedence for a library name:

1. Architecture-specific override in the `[Import]` attribute itself.
2. Architecture-specific override in `writ.toml` `[libraries.<name>]`.
3. `default` in `writ.toml` `[libraries.<name>]`.
4. The logical name from the `[Import]` positional argument, as-is.

The `[libraries]` section is entirely optional. Projects that specify all overrides in `[Import]` attributes do not need
it. Projects that distribute pre-compiled artifacts without source can rely solely on attribute-level overrides.

## 1.2.5 Conditions

The optional `[conditions]` section defines named conditions for conditional compilation (
see [Section 1.17.4](#1174-conditional-compilation)). Each key is a condition name and its value is a boolean indicating
whether the condition is active.

```toml
[conditions]
debug = true
playstation = false
xbox = false
editor = true
```

Conditions can also be set or overridden via compiler flags, allowing build scripts to control platform targeting
without modifying `writ.toml`:

```bash
writc --condition playstation=true --condition debug=false
```

Compiler flags take precedence over `writ.toml` values. A condition referenced in a `[Conditional("name")]` attribute
that is not defined in either `writ.toml` or compiler flags is treated as inactive (false).

## 1.2.6 Locale Identifiers

Locale identifiers follow [BCP 47](https://www.rfc-editor.org/info/bcp47) language tags. Common examples: `en`, `de`,
`fr`, `ja`, `ko`, `zh`, `pt-BR`, `en-GB`. The `default` locale is the language used for inline dialogue text in `.writ`
source files.

The `supported` array lists all locales the project targets. If omitted, only the `default` locale is assumed. The
`writ loc export` tool uses this list to generate CSV column headers.

## 1.2.7 Build Profiles

The optional `[profile.debug]` and `[profile.release]` sections configure compilation behavior for each build mode.
The `writ build` command selects a profile via `--release` or `--debug` flags; the default is `debug`.

| Field | Type | Default (debug) | Default (release) | Effect |
|-------|------|-----------------|-------------------|--------|
| `debug_info` | bool | `true` | `false` | When true, the compiled module includes `DebugLocal` entries mapping registers to variable names and source positions. When false, these entries are omitted to reduce module size. |

Additional profile fields may be added in future versions for optimization and stripping settings. Fields not recognized
by the current compiler are silently ignored.

Output path: `{compiler.output}/{profile}/{project.name}.writc`. The default output base is `build/`. Directories are
created automatically if they do not exist.

Example:

```toml
[profile.debug]
debug_info = true

[profile.release]
debug_info = false
```

## 1.2.8 Dependencies

The optional `[dependencies]` section declares pre-compiled Writ library modules that the compiler loads at compile
time. Each entry maps a dependency name to a path pointing at a `.writc` binary module produced by `writ build`.

**Simple form** (path string):

```toml
[dependencies]
writ-std = "path/to/writ-std.writc"
```

**Detailed form** (inline table with `path` key):

```toml
[dependencies]
writ-std = { path = "path/to/writ-std.writc" }
```

Both forms are equivalent. Use the detailed form when future dependency fields (e.g., version pinning) are needed.

Rules:

- Paths are relative to the project root (the directory that contains `writ.toml`).
- Each dependency must be a valid `.writc` binary module compiled with `writ build`.
- The dependency name (e.g., `writ-std`) is used only for error messages and diagnostics; it does not affect
  how types are qualified in source code.
- A project may declare any number of dependencies.

## 1.2.9 Cross-Module Type Resolution

When the compiler encounters a type name (such as `List<int>`) that is not defined in the current compilation unit, it
checks the types loaded from dependency modules declared in `[dependencies]`.

### Visibility and namespace

Library types are visible using their fully-qualified names at all times. If a library type belongs to a namespace
(e.g., `collections::List`), the user must do one of the following:

- Use the fully-qualified name directly:
  ```writ
  let list: collections::List<int> = collections::List::new();
  ```

- Add a `using` declaration to bring the namespace into scope:
  ```writ
  using collections;
  let list: List<int> = List::new();
  ```

If a library type has no namespace (i.e., it is declared at the top level of the library module), it is visible without
any `using` declaration.

### Resolution rules

Library types participate in the same name resolution rules as locally-defined types:

- A `using` declaration for a namespace that exists in a dependency module imports all types from that namespace into
  the local unqualified scope, exactly as it would for locally-defined namespaces.
- A library type that is visible in scope may be used as a field type, parameter type, return type, generic argument,
  or in any other type position.
- Method calls, field accesses, and contract implementations on library types are resolved using the method signatures
  stored in the dependency module's binary tables.

### Duplicate definitions

If a user source file declares a type whose fully-qualified name matches a type already loaded from a dependency
module, the compiler reports a `E0001 duplicate definition` error. The error message identifies the dependency module
by name (as declared in `writ.toml [dependencies]`):

```
error[E0001]: type `List` is already defined in dependency module `writ-std`
  --> src/main.writ:5:1
   |
 5 | pub class List<T> { ... }
   | ^^^^^^^^^^^^^^^^^^^^^^^^^
```

User code cannot shadow a library type; if a name collision is intentional (e.g., a local reimplementation), rename
the local type.

## 1.2.10 Virtual Module Types

The Writ runtime provides a built-in virtual module that is available in every compilation unit without any explicit
`[dependencies]` declaration. The virtual module contains:

**Standard contracts** (arithmetic, comparison, collections, reflection):

- `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg` — arithmetic operators
- `Not` — logical/bitwise negation
- `Eq`, `Ord` — equality and ordering
- `BitAnd`, `BitOr` — bitwise operators
- `Index`, `IndexSet` — indexing operators
- `Iterable`, `Iterator` — iteration protocol
- `Into` — type conversion
- `Error` — error value contract
- `Hashable` — hashing contract
- `Reflectable` — runtime type reflection

**Built-in generic types**:

- `Option<T>` — nullable wrapper (`T?` desugars to `Option<T>`)
- `Result<T, E>` — fallible return value
- `ChoiceOption` — dialogue choice item

These types follow the same resolution mechanism as user library types. They are resolved through the same DefMap
lookup used for dependency modules, so `using` declarations work uniformly across virtual module types and user library
types.

---

