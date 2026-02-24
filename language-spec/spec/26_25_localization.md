# 1. Writ Language Specification
## 25. Localization

Writ provides a localization system designed around standard game industry workflows. Dialogue text remains clean in
source files — localization is handled by compiler tooling and the runtime string table, not by language syntax.

### 25.1 Overview

The localization system has two tiers:

1. **String-level translation (Tier 1):** The compiler extracts all dialogue strings into CSV files. Translators (human
   or automated) fill in translations. The runtime performs string table lookups at display time. This covers ~95% of
   localization needs.

2. **Structural overrides (Tier 2):** When a locale requires different dialogue *structure* (different choices,
   different branching, different number of lines), a `[Locale]` attribute marks an entire `dlg` as a locale-specific
   replacement.

### 25.2 String Extraction & the Localization Key

Every dialogue line and choice label in a `dlg` block is assigned a **localization key**. By default, this key is a
hex-encoded hash computed from a composite string that uniquely identifies the occurrence. Lines may also specify a
manual key with `#key` (see [Section 13.7](#137-localization-keys)), which overrides the computed hash.

#### 25.2.1 Key Computation

The localization key is computed using the **FNV-1a 32-bit** hash algorithm over the following input string:

```
input = namespace + "\0" + method + "\0" + speaker + "\0" + content + "\0" + occurrence_index
```

Where:

- `namespace` — the fully qualified namespace of the `dlg` (e.g., `dialogue`)
- `method` — the `dlg` function name (e.g., `greetPlayer`)
- `speaker` — the resolved speaker name for dialogue lines (e.g., `Narrator`, `OldTim`), or the empty string `""` for
  choice labels
- `content` — the raw text content of the line or choice label, with interpolation slots preserved literally (e.g.,
  `Hey, {name}.`)
- `occurrence_index` — a zero-based index distinguishing duplicate occurrences of the same
  `(namespace, method, speaker, content)` tuple within a single `dlg`. For the first occurrence, this is `"0"`. For the
  second, `"1"`, etc.
- `\0` — the null byte, used as a field separator

The hash output is rendered as an 8-character lowercase hexadecimal string (e.g., `a3f7c012`).

#### 25.2.2 FNV-1a 32-bit Algorithm

The algorithm is specified exactly to ensure all Writ implementations produce identical keys:

```
OFFSET_BASIS = 0x811c9dc5  (2166136261)
PRIME        = 0x01000193  (16777619)
MASK         = 0xFFFFFFFF  (2^32 - 1)

fn fnv1a_32(data: byte[]) -> uint32:
    hash = OFFSET_BASIS
    for each byte in data:
        hash = hash XOR byte
        hash = (hash * PRIME) AND MASK
    return hash
```

The input string is encoded as UTF-8 bytes before hashing.

#### 25.2.3 Deduplication Index

The `occurrence_index` field solves the deduplication problem. Consider:

```
dlg battleTalk {
    @Warrior Yes, please!
    @Healer Yes, please!
}
```

Without the deduplication index, these two lines would produce different keys because the `speaker` field differs (
`Warrior` vs `Healer`). However, if the *same* speaker says the *same* line twice in the same `dlg`:

```
dlg annoyingNPC {
    @Guard Move along.
    @Guard Move along.
}
```

The first occurrence gets `occurrence_index = "0"`, the second gets `occurrence_index = "1"`. This ensures each line
gets a unique key even in pathological cases.

### 25.3 CSV Localization Format

The `writ loc export` command produces CSV files for translator workflows. The format is specified exactly to ensure
interoperability.

#### 25.3.1 CSV Encoding Rules

| Property         | Value                                                                                                                                                              |
|------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Encoding         | UTF-8 (BOM optional, recommended for Excel compatibility)                                                                                                          |
| Delimiter        | Comma (`,`, U+002C)                                                                                                                                                |
| Quote character  | Double quote (`"`, U+0022)                                                                                                                                         |
| Quoting rule     | A field MUST be quoted if it contains a comma, a double quote, or a newline (CR, LF, or CRLF). A field MAY be quoted even if it does not contain these characters. |
| Escape rule      | A double quote within a quoted field is escaped by doubling it: `""`                                                                                               |
| Line endings     | CRLF (`\r\n`) for maximum compatibility. Parsers MUST also accept LF (`\n`).                                                                                       |
| Header row       | Required. Must be the first row.                                                                                                                                   |
| Trailing newline | The file MUST end with a line ending after the last data row.                                                                                                      |

#### 25.3.2 Column Structure

The CSV has the following columns, in order:

| Column           | Description                                                                                                                                                     |
|------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Key`            | The 8-character hex FNV-1a hash (localization key).                                                                                                             |
| `Namespace`      | The namespace of the source `dlg`.                                                                                                                              |
| `Method`         | The `dlg` function name.                                                                                                                                        |
| `Speaker`        | The resolved speaker name, or empty for choice labels.                                                                                                          |
| `Context`        | One of: `line` (dialogue line), `choice` (choice label).                                                                                                        |
| `Default`        | The default locale text, with interpolation slots preserved literally (e.g., `Hey, {name}.`).                                                                   |
| *locale columns* | One column per locale listed in `writ.toml`'s `locale.supported` array, excluding the default locale. Column header is the BCP 47 tag (e.g., `de`, `fr`, `ja`). |

#### 25.3.3 Example

Given `writ.toml`:

```toml
[locale]
default = "en"
supported = ["en", "de", "ja"]
```

And source:

```
// dialogue/greet.writ
namespace dialogue;

dlg greetPlayer(name: string) {
    @Narrator Hey, {name}.
    @Narrator How are you?
    $ choice {
        "I'm great!" {
            Wonderful!
        }
        "Not so good" {
            Sorry to hear that.
        }
    }
}
```

The exported CSV (`writ loc export`):

```csv
Key,Namespace,Method,Speaker,Context,Default,de,ja
a3f7c012,dialogue,greetPlayer,Narrator,line,"Hey, {name}.",,
b92e1d44,dialogue,greetPlayer,Narrator,line,How are you?,,
c71a8f30,dialogue,greetPlayer,,choice,I'm great!,,
d604b2e8,dialogue,greetPlayer,Narrator,line,Wonderful!,,
e5190ca1,dialogue,greetPlayer,,choice,Not so good,,
f8a23d77,dialogue,greetPlayer,Narrator,line,Sorry to hear that.,,
```

Translators fill in the `de` and `ja` columns. The completed CSV is imported via `writ loc import`, which produces the
runtime string table.

#### 25.3.4 Interpolation Slot Validation

When importing a translated CSV, the compiler MUST verify that every interpolation slot present in the `Default` column
also appears in each translation. Missing or extra slots produce a compile error:

```
Error: locale "de", key "a3f7c012": interpolation slot {name} missing in translation
```

The order of interpolation slots MAY differ between languages (to accommodate different grammar). Only the presence of
the same set of slot names is checked.

### 25.4 Compiler Tooling

The following commands are part of the Writ compiler toolchain:

| Command           | Description                                                                                                                                                                                                             |
|-------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `writ loc export` | Extracts all dialogue strings from all `dlg` blocks and produces a CSV file. Locale columns are sourced from `writ.toml`. If a CSV already exists, new strings are appended and removed strings are marked (see below). |
| `writ loc check`  | Validates a translated CSV against the current source. Reports: missing translations (new source strings not in CSV), orphaned translations (CSV keys that no longer exist in source), interpolation slot mismatches.   |
| `writ loc import` | Reads a completed CSV and produces the runtime string table in the format required by the target runtime (binary, JSON, etc. — runtime-specific).                                                                       |

#### 25.4.1 Incremental Export

When `writ loc export` is run against an existing CSV file:

- New strings (present in source but not in CSV) are appended at the end.
- Removed strings (present in CSV but not in source) are NOT deleted. Instead, they are kept in place, and
  `writ loc check` reports them as orphaned. This prevents accidental loss of completed translations during refactoring.
- Modified strings (same key but different `Default` text — this should not happen with content-hashed keys, but can
  occur if the hashing input changes due to eg. a speaker rename) are reported as conflicts.

### 25.5 Runtime String Table Lookup

At runtime, the `say(speaker, text)` function performs the following lookup:

1. Compute the localization key for the text (using the same FNV-1a algorithm).
2. Look up the key in the active locale's string table.
3. If a translation is found, use it (after interpolation slot substitution).
4. If no translation is found, use the original text as fallback.

This means the default locale text is always the fallback. Games in development can run entirely without a string
table — the inline text just works.

> **Note:** The runtime must have access to the same key computation inputs (namespace, method, speaker, occurrence
> index) to perform the lookup. The compiler embeds this metadata in the lowered `say()` calls. Alternatively, the
> compiler can pre-compute keys and emit `say_localized(key, fallback_text, speaker)` calls.

### 25.6 Structural Overrides with [Locale]

When a locale requires fundamentally different dialogue structure — different choices, different branching, additional
or fewer lines — a `[Locale]` override replaces the entire `dlg` block for that locale.

#### 25.6.1 Syntax

```
// Default (en) version
dlg greetPlayer(name: string) {
    @Narrator Hey, {name}.
    $ choice {
        "I'm great!" {
            Wonderful!
        }
        "Not so good" {
            Sorry to hear that.
        }
    }
}

// Japanese structural override — different choice structure
[Locale("ja")]
dlg greetPlayer(name: string) {
    @Narrator {name}さん、こんにちは。
    $ choice {
        "丁寧に話す" {
            @Narrator かしこまりました。
        }
        "カジュアルに話す" {
            @Narrator いいね！
        }
        "黙る" {
            @Narrator ...
            -> silentExit
        }
    }
}
```

#### 25.6.2 Rules

1. The `[Locale(tag)]` attribute takes a single BCP 47 locale string matching one of the `locale.supported` entries in
   `writ.toml`.
2. The `dlg` name and parameter signature MUST exactly match the default version. The compiler enforces this.
3. At most one `[Locale]` override per locale per `dlg`. Duplicates are a compile error.
4. When a `[Locale]` override exists for a given locale, the Tier 1 string table entries for that `dlg` in that locale
   are **ignored** by the runtime. The override is the complete replacement.
5. The override's own dialogue strings are extracted into the localization CSV like any other `dlg`. This allows a
   `[Locale("ja")]` override to itself be translated to other locales if desired (rare but possible).
6. A `[Locale]` override may use `->` transitions to different `dlg` blocks than the default version. The override has
   full freedom in its dialogue structure.
7. There is no `[Locale]` attribute for the default locale. The un-attributed `dlg` IS the default locale version.

#### 25.6.3 Runtime Dispatch

When the runtime calls a `dlg` function, it checks:

1. Is there a `[Locale]` override for the active locale? → Use the override.
2. Otherwise → Use the default version (with Tier 1 string table lookup for individual lines).

This is a simple dispatch — the compiler generates a lookup table of `(dlg_name, locale) → function_pointer` for all
overrides.

### 25.7 Design Rationale

**Why CSV?** CSV is the lowest common denominator. Every spreadsheet application, every programming language, and every
translation management system can read and write CSV. Small developers can pipe the `Default` column through DeepL or
Google Translate with a few lines of Python. Large studios can import it into their existing localization pipeline. No
custom tooling required on the translator side.

**Why content-hashed keys?** Inserting, removing, or reordering dialogue lines does not invalidate existing
translations. Only changing the actual text content (or renaming a speaker) produces a new key, which is correct — the
translation needs updating in that case.

**Why FNV-1a?** It is trivially implementable (~10 lines of code in any language), well-specified, produces reasonably
distributed 32-bit hashes, and has no licensing or patent concerns. SHA-256 or similar would work but is overkill for
non-security-critical string table keys.

**Why structural overrides instead of per-line locale blocks?** Per-line annotations (`[de] Wie geht's?`) become
unreadable with more than 2-3 locales. Structural overrides are rare (~5% of dialogues) and justify their own `dlg`
block. The clean separation keeps the default-locale authoring experience uncluttered.

---

