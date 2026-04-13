# Editor Setup

Writ has first-class VS Code support with language intelligence and debugging.

## Installing the Extension

Search for **Writ Language** in the VS Code extension marketplace, or install from a `.vsix` package:

```bash
code --install-extension writ-lang-0.1.1.vsix
```

The extension bundles the `writ-lsp` and `writ-dap` binaries. To use custom builds (e.g. built from source), set the `writ.serverPath` setting to the directory containing your binaries.

## Language Features

Once installed, `.writ` files get full language intelligence:

- **Diagnostics** -- real-time errors and warnings as you type
- **Go to Definition** -- jump to any symbol's declaration
- **Find References** -- locate all usages across your project
- **Hover** -- view type signatures and documentation
- **Completions** -- context-aware suggestions for identifiers, keywords, fields, and namespace members (`log::`, `Option::`, `Result::`, enum variants)
- **Signature Help** -- parameter hints while typing function calls

## Semantic Highlighting

The extension provides semantic token coloring beyond standard syntax highlighting:

| Token Kind | Color |
|------------|-------|
| Entities | teal |
| Types | blue |
| Components | light blue |
| Dialogue speakers | orange |

## Setting Up Debugging

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

Or use the **Writ: Launch Current File** snippet from the debugger panel.

For multi-file projects, point `program` at your `writ.toml`:

```json
{
  "type": "writ",
  "request": "launch",
  "name": "Launch Project",
  "program": "${workspaceFolder}/writ.toml"
}
```

See the [Debugging](../tooling/debugging.md) chapter for a full walkthrough of breakpoints, stepping, and variable inspection.

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `writ.serverPath` | `""` | Path to a directory containing `writ-lsp` and `writ-dap` binaries. Leave empty to use the bundled binaries. |
