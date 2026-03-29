# Hello World

Create your first Writ program.

## Writing the Program

Create a file named `hello.writ` with the following content:

```writ
entity Narrator {}

pub fn main() {
    let speaker: Entity = Entity.getOrCreate<Narrator>();
    ::say(speaker, "Hello, World!");
}
```

**What this does:**

- `entity Narrator {}` defines a named entity type used as a dialogue speaker.
- `Entity.getOrCreate<Narrator>()` retrieves the singleton `Narrator` instance,
  creating it if it does not yet exist.
- `::say(speaker, text)` is a built-in dialogue function that outputs a line of
  speech attributed to the given speaker entity.

## Compiling

Compile the source file to a binary `.writc` module:

```bash
writ compile hello.writ
```

This produces `hello.writc` in the same directory.

## Running

Execute the compiled module:

```bash
writ run hello.writc
```

Expected output:

```
[say] <entity@0>: Hello, World!
```

```admonish note
`[say]` is the annotation format used by the CLI host. Real game hosts provide their
own display logic for spoken dialogue. The speaker is shown as `<entity@0>` because
`Entity` values are opaque handles in the CLI — the handle index is printed, not the
type name.
```

```admonish warning
`writ run` takes a compiled `.writc` binary, not a `.writ` source file. Always compile
first with `writ compile`, then run the resulting `.writc` output.
```
