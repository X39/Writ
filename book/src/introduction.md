# Writ Language

**Draft v0.5** --- March 2026

A game scripting language with first-class dialogue support, C-style scripting,
a Rust-inspired type system, and an entity-component architecture.

**File extension:** `.writ`

---

This documentation covers:

- **Getting Started** --- Installation, your first program, editor setup, and CLI reference
- **Tutorials** --- Guided walkthroughs: dialogue, entities, and building a quest system
- **Language Reference** --- The complete Writ language specification (syntax, types, entities, dialogue, concurrency, and more)
- **Tooling** --- VS Code extension and DAP debugging
- **Embedding Writ** --- Integrating the Writ runtime into a host application
- **Architecture** --- Compiler pipeline overview and crate dependency map
- **IL Specification** --- The Writ Intermediate Language: virtual machine, instruction set, module format, execution model

```admonish note
Writ is a statically-typed game scripting language designed for dialogue-heavy games.
If you are new to Writ, start with the [Installation](getting-started/installation.md) guide,
then work through the [Tutorials](tutorials/first-dialogue.md).
```

```admonish warning
Writ is pre-1.0. Breaking changes may occur between versions.
```
