# Debugging with DAP

Writ includes a Debug Adapter Protocol (DAP) server that integrates with VS Code's debugger. This chapter covers setting up and using the debugger.

## Launch Configuration

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

For multi-file projects using `writ.toml`, point `program` at the project file:

```json
{
  "type": "writ",
  "request": "launch",
  "name": "Launch Project",
  "program": "${workspaceFolder}/writ.toml"
}
```

## Breakpoints

Click in the gutter next to any line to set a breakpoint. The debugger supports:

- **Line breakpoints** -- pause execution at a specific line
- **Conditional breakpoints** -- break only when a condition is true
- **Hit count breakpoints** -- break after N hits

When a breakpoint is hit, execution pauses and the editor highlights the current line.

## Stepping

Once paused, use the standard stepping controls:

| Action | Shortcut | Description |
|--------|----------|-------------|
| Step Over | `F10` | Execute the current line, stepping over function calls |
| Step Into | `F11` | Step into the function call on the current line |
| Step Out | `Shift+F11` | Run until the current function returns |
| Continue | `F5` | Resume execution until the next breakpoint |

## Inspecting State

### Call Stack

The **Call Stack** panel shows the chain of function calls that led to the current point. Click any frame to navigate to that location in source.

### Variables

The **Variables** panel shows all local variables and parameters in the current scope with their types and values.

### Watch Expressions

Add expressions to the **Watch** panel to evaluate them at each pause point.

## Crash Debugging

When a runtime crash occurs (e.g. unwrapping `None`, array out of bounds), the debugger halts at the crash site with **break-before-unwind** behavior:

- The full call stack is preserved
- All local variables are inspectable at their last values
- A crash stacktrace with source locations is displayed in the debug console

This makes it straightforward to diagnose the state that led to the crash without needing to reproduce it with breakpoints.
