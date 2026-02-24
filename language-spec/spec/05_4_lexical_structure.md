# 1. Writ Language Specification
## 4. Lexical Structure

### 4.1 Keywords

| Category              | Keywords                                                                                                 |
|-----------------------|----------------------------------------------------------------------------------------------------------|
| Declarations          | `fn`, `dlg`, `struct`, `enum`, `contract`, `impl`, `entity`, `component`, `namespace`, `extern`, `using` |
| Visibility            | `pub`, `priv`                                                                                            |
| Variables             | `let`, `mut`, `const`, `global`                                                                          |
| Control flow          | `if`, `else`, `match`, `for`, `while`, `in`, `return`, `break`, `continue`                               |
| Concurrency           | `spawn`, `detached`, `join`, `cancel`, `defer`                                                           |
| Error handling        | `try`                                                                                                    |
| Types                 | `void`                                                                                                   |
| Values                | `true`, `false`, `null`, `self`                                                                          |
| Entity                | `use`, `on`                                                                                              |
| Concurrency (globals) | `atomic`                                                                                                 |

> **Context-sensitive keywords:** `entity`, `component`, `use`, and `on` are reserved in declaration context
> (where they begin a declaration) but may be used as identifiers in expression context. For example, a local
> variable named `use` or a function parameter named `on` is valid.

### 4.2 Sigils & Delimiters

| Sigil | Context       | Meaning                                                                    |
|-------|---------------|----------------------------------------------------------------------------|
| `@`   | dlg           | Speaker switch / inline speaker attribution                                |
| `$`   | dlg           | Escape into code (statement, block, or dialogue conditional/choice)        |
| `#`   | dlg           | Manual localization key annotation (`#key` at end of line)                 |
| `$`   | string        | Formattable string prefix (enables `{expr}` interpolation)                 |
| `"""` | string        | Raw string delimiter (no escape processing, multi-line)                    |
| `->`  | dlg           | Terminal dialogue transition (tail call)                                   |
| `?`   | expression    | Null propagation (Option chaining)                                         |
| `!`   | expression    | Unwrap-or-crash (Option and Result)                                        |
| `try` | expression    | Result error propagation                                                   |
| `::`  | expression    | Namespace / enum variant access; leading `::` resolves from root namespace |
| `{ }` | all           | Block delimiters                                                           |
| `;`   | fn / $ escape | Statement terminator                                                       |
| `[ ]` | declaration   | Attributes (before declarations)                                           |
| `[ ]` | expression    | Array literal, indexing, component access                                  |
| `T[]` | type          | Array type (postfix notation)                                              |
| `..`  | expression    | Exclusive-end range (`0..10`)                                              |
| `..=` | expression    | Inclusive-end range (`0..=10`)                                             |
| `^`   | inside `[]`   | From-end index (`^1` = last element)                                       |

### 4.3 Comments

```
// Single-line comment

/* Multi-line
   comment */
```

### 4.4 String Literals

Writ has four string literal forms, built from two orthogonal axes: **basic vs raw** and **plain vs formattable**.

#### 4.4.1 Basic Strings

Delimited by `"..."`. Support escape sequences but **not** interpolation.

```
let name = "Alice";
let greeting = "Hello, world!\nWelcome.";
let path = "C:\\Users\\data";
```

**Escape sequences (basic strings only):**

| Escape       | Meaning                                                                  |
|--------------|--------------------------------------------------------------------------|
| `\\`         | Literal backslash                                                        |
| `\n`         | Newline                                                                  |
| `\t`         | Tab                                                                      |
| `\r`         | Carriage return                                                          |
| `\0`         | Null character                                                           |
| `\"`         | Literal double quote                                                     |
| `\u{XXXX}`   | Unicode codepoint (1–6 hex digits)                                       |
| `\` (at EOL) | Line continuation (joined with single space, leading whitespace trimmed) |

#### 4.4.2 Formattable Strings

Prefixed with `$`. Enables interpolation via `{expr}` inside the string. Each interpolated expression is converted to a
string by implicitly calling `.into<string>()` (requires an `Into<string>` implementation; see Section 10.2).

```
let name = "Alice";
let msg = $"Hello, {name}!";
let dmg = $"Took {base * modifier} damage.";
```

To include a literal brace, double it:

```
let json = $"{{\"name\": \"{name}\"}}";
// Result: {"name": "Alice"}
```

Interpolated expressions may be any valid expression, including nested formattable strings (`$"outer {$"inner {x}"}"`),
`if`/`else`, `match`, blocks, lambdas, and method chains.

Formattable strings support all the same escape sequences as basic strings.

#### 4.4.3 Raw Strings

Delimited by `"""..."""`. No escape sequences are processed — content is taken verbatim. May span multiple lines. The
opening `"""` must be followed by a newline; the closing `"""` must appear on its own line. Leading common whitespace is
stripped (dedented).

```
let text = """
    This is raw text.
    No \n escaping happens here.
    Backslashes are literal: C:\Users\data
    """;
```

To include `"""` inside a raw string, add additional `"` characters to both the opening and closing delimiters. The
closing delimiter must use the same number of quotes as the opening:

```
let nested = """"
    This raw string can contain """ inside it.
    """";
```

Five quotes to embed four:

```
let deep = """""
    Contains both """ and """" inside.
    """"";
```

The rule: a raw string opened with N quotes (where N >= 3) is closed by exactly N consecutive quotes.

#### 4.4.4 Formattable Raw Strings

Prefixed with `$` and delimited by `"""..."""`. Combines raw string semantics (no escape processing, multi-line,
dedented) with interpolation.

```
let report = $"""
    Quest: {quest.name}
    Status: {quest.status}
    Reward: {quest.gold} gold
    """;
```

Literal braces use doubling, same as formattable strings:

```
let json = $"""
    {{"name": "{player.name}", "level": {player.level}}}
    """;
```

Formattable raw strings also support additional `"` delimiters for embedding `"""`:

```
let example = $""""
    Template with """ triple quotes and {expr} interpolation.
    """";
```

#### 4.4.5 Dialogue Lines

In `dlg` blocks, text lines (after speaker attribution or as continuation lines) are implicitly formattable. They do not
use quotes, and their boundary is end-of-line. Interpolation with `{expr}` is always available. Escape sequences from
basic strings are recognized. See [Section 13](#13-dialogue-blocks-dlg) for full details.

```
dlg greet(name: string) {
    @Narrator Hello, {name}. Welcome to the world.
}
```

#### 4.4.6 Runtime Type

All four string literal forms produce values of type `string`. There is no distinct type for formattable vs basic — the
`$` prefix and `"""` delimiters control compile-time parsing behavior only.

---

