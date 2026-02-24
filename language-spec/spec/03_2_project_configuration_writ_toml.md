# 1. Writ Language Specification
## 2. Project Configuration (writ.toml)

Every Writ project requires a `writ.toml` file at the project root. This file defines project metadata, compiler
settings, and localization configuration.

### 2.1 Format

The file uses [TOML v1.0](https://toml.io/en/v1.0.0) syntax.

### 2.2 Required Fields

```toml
[project]
name = "my-game"
version = "0.1.0"

[locale]
default = "en"
```

### 2.3 Optional Fields

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

### 2.4 Library Resolution

The optional `[libraries.<name>]` sections map logical library names (as used in `[Import("name")]` attributes) to
architecture-specific library names. Each entry supports a `default` key and architecture-specific overrides using the
identifiers defined in [Section 24.3](#243-architecture-identifiers).

Resolution precedence for a library name:

1. Architecture-specific override in the `[Import]` attribute itself.
2. Architecture-specific override in `writ.toml` `[libraries.<name>]`.
3. `default` in `writ.toml` `[libraries.<name>]`.
4. The logical name from the `[Import]` positional argument, as-is.

The `[libraries]` section is entirely optional. Projects that specify all overrides in `[Import]` attributes do not need
it. Projects that distribute pre-compiled artifacts without source can rely solely on attribute-level overrides.

### 2.5 Conditions

The optional `[conditions]` section defines named conditions for conditional compilation (
see [Section 16.4](#164-conditional-compilation)). Each key is a condition name and its value is a boolean indicating
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

```
writc --condition playstation=true --condition debug=false
```

Compiler flags take precedence over `writ.toml` values. A condition referenced in a `[Conditional("name")]` attribute
that is not defined in either `writ.toml` or compiler flags is treated as inactive (false).

### 2.6 Locale Identifiers

Locale identifiers follow [BCP 47](https://www.rfc-editor.org/info/bcp47) language tags. Common examples: `en`, `de`,
`fr`, `ja`, `ko`, `zh`, `pt-BR`, `en-GB`. The `default` locale is the language used for inline dialogue text in `.writ`
source files.

The `supported` array lists all locales the project targets. If omitted, only the `default` locale is assumed. The
`writ loc export` tool uses this list to generate CSV column headers.

---

