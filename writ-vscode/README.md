# Writ Language Support

Full-featured VS Code support for the Writ game scripting language, including language server integration and DAP debugging.

## Features

### Language Intelligence

- **Diagnostics** -- real-time error and warning reporting as you type
- **Go to Definition** -- jump to the declaration of any symbol
- **Find References** -- locate all usages of a symbol across your project
- **Hover Information** -- view type signatures and documentation on hover
- **Completions** -- context-aware autocompletion for identifiers, keywords, and fields
- **Namespace completions** -- auto-complete for `log::`, `Option::`, `Result::`, and user-defined enum variants
- **New-keyword completions** -- context-aware type suggestions after `new` keyword (constructable types only)
- **Signature Help** -- parameter hints while typing function calls
- **Crash stacktraces** -- full stack trace with source locations when runtime crashes occur

### Syntax & Semantic Highlighting

- TextMate grammar for Writ syntax highlighting
- Semantic token support with distinct colors for:
  - **Entities** (teal)
  - **Types** (blue)
  - **Components** (light blue)
  - **Dialogue speakers** (orange)

### Debugging (DAP)

- Breakpoints
- Step over, step into, step out
- Call stack inspection
- Variable and watch expression evaluation
- Launch configuration snippets
- Multi-file project debugging (launch via writ.toml)
- Crash halt with live variable inspection (break-before-unwind)

## Requirements

- **VS Code** 1.74 or later
- Bundled `writ-lsp` and `writ-dap` binaries are included in the extension package. To use custom builds, set the `writ.serverPath` setting.

## Extension Settings

| Setting           | Default | Description                                                                 |
|-------------------|---------|-----------------------------------------------------------------------------|
| `writ.serverPath` | `""`    | Path to a directory containing `writ-lsp` and `writ-dap` binaries. Leave empty to use the bundled binaries. |

## Debugging

Add a launch configuration to `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "writ",
      "request": "launch",
      "name": "Launch Writ Program",
      "program": "${workspaceFolder}/${file}"
    }
  ]
}
```

Or use the **Writ: Launch Current File** configuration snippet from the debugger panel.

## Known Issues

See the [issue tracker](https://github.com/X39/Writ/issues) for known issues.

## Release Notes

### 0.1.1

- Namespace completions for `log::`, `Option::`, `Result::`, and user-defined enums
- Context-aware completions after `new` keyword (constructable types only)
- Full crash stacktrace display with source locations
- Multi-file writ.toml project debugging
- Crash halt with break-before-unwind and live variable inspection
- Various DAP reliability fixes (breakpoint alignment, scopes, variable names)

### 0.1.0

Initial release:

- TextMate grammar and language configuration
- LSP client with diagnostics, go-to-definition, hover, references, completions, and signature help
- Semantic token support for entity, type, component, and speaker tokens
- DAP client with breakpoints, stepping, call stack, and variable inspection
- Bundled binary support with configurable `writ.serverPath`

## License

LGPL-3.0-only — see LICENSE in the repository root.
