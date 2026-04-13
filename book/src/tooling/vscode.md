# VS Code Extension

The Writ VS Code extension provides full language server and debug adapter integration. This chapter covers the complete feature set -- for a quick setup guide, see [Editor Setup](../getting-started/editor-setup.md).

## Language Intelligence

### Diagnostics

The language server reports errors and warnings in real-time as you type. Diagnostics cover parse errors, type mismatches, unknown symbols, and more.

### Go to Definition

Jump to the declaration of any symbol with `F12` or `Ctrl+Click`. Works across files in multi-file projects.

### Find References

Locate all usages of a symbol across your project with `Shift+F12`.

### Hover Information

Hover over any symbol to see its type signature and documentation.

### Completions

Context-aware autocompletion includes:

- **Identifiers** -- local variables, parameters, functions, types
- **Keywords** -- contextual keyword suggestions
- **Fields** -- struct and class field access after `.`
- **Namespace members** -- `log::`, `Option::`, `Result::`, and user-defined enum variants after `::`
- **Constructable types** -- type suggestions after the `new` keyword (only types that can be constructed)

### Signature Help

Parameter hints appear automatically when typing function call arguments, showing the expected parameter name and type.

### Crash Stacktraces

When a runtime crash occurs during debugging, the extension displays a full stack trace with source locations, making it easy to trace the cause.

## Syntax and Semantic Highlighting

The extension provides two layers of highlighting:

1. **TextMate grammar** -- standard syntax highlighting for keywords, strings, comments, and operators
2. **Semantic tokens** -- LSP-powered coloring that distinguishes language constructs by role:

| Token Kind | Color | Example |
|------------|-------|---------|
| Entity types | teal | `Guard`, `Narrator` |
| Types | blue | `int`, `string`, `Vec2` |
| Components | light blue | `Speaker`, `Collider` |
| Dialogue speakers | orange | `@Narrator`, `@guard` |

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `writ.serverPath` | `""` | Path to a directory containing `writ-lsp` and `writ-dap` binaries. Leave empty for bundled binaries. |
