# Writ Language Specification
## 13. Dialogue Blocks (dlg)

Dialogue blocks are the primary authoring construct for game dialogue. They provide a specialized syntax where plain
text lines are the default and code requires explicit escaping. All `dlg` blocks lower to `fn` calls at compile time.

### 13.1 Basic Syntax

```
dlg greetPlayer(playerName: string) {
    @Narrator Hey there, {playerName}.
    @Narrator
    How are you today?
    I hope you're doing well.
}
```

### 13.2 Speaker Attribution

The `@` sigil controls speaker attribution. It has two forms:

- `@speaker text` — Inline form. Attributes a single line to the speaker.
- `@speaker` (on its own line) — Sets the active speaker for all subsequent lines until the next `@` or end of block.

Speaker resolution for `@` in dialogue:

1. Check local variables and parameters → use that instance directly.
2. Check `[Singleton]` entities with a `Speaker` component → resolve via `Entity.getOrCreate<T>()`.
3. Otherwise → compile error: unknown speaker.

```
dlg shopScene(customer: Entity, guard: Guard) {
    @Narrator You enter the shop.        // singleton, auto-resolved
    @OldTim Welcome, traveler!            // singleton, auto-resolved
    @guard Halt!                          // parameter, direct reference
    @customer Who, me?                    // parameter, direct reference
}
```

### 13.3 The $ Sigil in Dialogue

The `$` sigil is the escape from dialogue into code. It has four forms, disambiguated by the token following `$`:

| Form                 | Syntax             | Behavior                                       |
|----------------------|--------------------|------------------------------------------------|
| Single statement     | `$ statement;`     | Execute one code statement                     |
| Code block           | `$ { ... }`        | Execute a code block (all code inside)         |
| Dialogue conditional | `$ if` / `$ match` | Condition is code, branches are dialogue       |
| Dialogue choice      | `$ choice`         | Present player choices (branches are dialogue) |

```
dlg example {
    @Narrator
    Let me check your reputation.
    $ let rep = getReputation();         // single statement
    $ {                                  // code block
        let mut rep = rep * modifier;
        if rep > 100 {
            unlockAchievement("famous");
        }
    }
    Your reputation is {rep}.
}
```

### 13.4 Choices

`$ choice` presents options to the player. Each option is a quoted string followed by a block. The blocks inside are
dialogue context — text lines, speaker attributions, and further `$` escapes all work. Choice labels require quotes
because they are not speaker-attributed and need a clear boundary before the block.

```
dlg shopkeeper {
    @OldTim
    What would you like?
    $ choice {
        "Buy something" {
            Let me show you my wares.
            $ openShopUI();
        }
        "Just looking" {
            Take your time.
        }
        "Goodbye" {
            Farewell, traveler.
        }
    }
}
```

### 13.5 Conditional Dialogue

`$ if` and `$ match` create dialogue-level conditionals. The condition or expression is code, but the branches remain in
dialogue context — unquoted text, `@speaker`, `$ choice`, and `->` all work inside the branches.

```
dlg greet(reputation: int) {
    @Narrator
    $ if reputation > 50 {
        You're quite famous around here.
    } else {
        I don't think I know you.
    }
    Either way, I have a task for you.
}
```

```
dlg questUpdate(status: QuestStatus) {
    @Narrator
    $ match status {
        QuestStatus::NotStarted => {
            I have a task for you, adventurer.
        }
        QuestStatus::InProgress => {
            How's that task coming along?
        }
        QuestStatus::Completed => {
            Well done! Here's your reward.
            $ giveReward();
        }
    }
}
```

Nesting is allowed — dialogue conditionals may contain `$ choice`, and choice branches may contain `$ if`:

```
dlg merchant(gold: int) {
    @OldTim Welcome!
    $ choice {
        "Show me your wares" {
            $ if gold < 10 {
                @OldTim You seem a bit short on coin.
            } else {
                @OldTim Here's what I have.
                $ openShopUI();
            }
        }
        "Goodbye" {
            Farewell, traveler.
        }
    }
}
```

### 13.6 Dialogue Transitions

The `->` operator performs a terminal transition to another dialogue. It is a tail call — execution does not return. It
must be the last statement in its block.

```
dlg questIntro {
    @Narrator A great evil threatens the land.
    $ choice {
        "Tell me more" {
            -> questDetails
        }
        "Not interested" {
            @Narrator Very well. Perhaps another time.
            -> townSquare
        }
    }
}
```

> **Note:** `->` is always terminal. For non-terminal dialogue invocation, call the lowered function directly via
`$ questDetails();`

### 13.7 Localization Keys

Dialogue lines are automatically assigned localization keys based on content hashing (
see [Section 25.2](#252-string-extraction--the-localization-key)). To assign a **stable manual key**, append `#key` at
the end of the line:

```
dlg greet(name: string) {
    @Narrator Hello, {name}. Welcome back. #greet_welcome
    @Narrator The world needs you. #greet_call_to_action
}
```

Manual keys override the auto-generated content hash. This prevents translation breakage when the default-locale text is
edited. Keys must be unique within a `dlg` block — duplicate `#key` values are a compile error.

Lines without `#key` continue to use the auto-generated FNV-1a hash as before. Choice labels also support `#key`:

```
$ choice {
    "Buy something" #shop_buy {
        ...
    }
    "Goodbye" #shop_goodbye {
        Farewell.
    }
}
```

### 13.8 Text Styling

Dialogue text may contain inline styling markup using BBCode-style tags: `[tag]...[/tag]`. The compiler treats these as
literal text — they pass through to the runtime's `say()` function, which interprets them.

**Recommended tag set:**

| Tag                    | Meaning                                                   |
|------------------------|-----------------------------------------------------------|
| `[b]...[/b]`           | Bold                                                      |
| `[i]...[/i]`           | Italic                                                    |
| `[color=X]...[/color]` | Text color (X is runtime-defined, e.g., `red`, `#FF0000`) |
| `[size=X]...[/size]`   | Text size (X is runtime-defined)                          |
| `[pause=N]`            | Pause for N milliseconds (self-closing)                   |

```
dlg warning {
    @Narrator This is [b]very important[/b].
    @Narrator The [color=red]dragon[/color] approaches!
    @Narrator ...[pause=1000] Run!
}
```

Runtimes may support additional tags (e.g., `[shake]`, `[wave]`, `[speed=slow]`) beyond the recommended set. Tags
unrecognized by a runtime should be stripped and the inner text displayed normally.

> **Note:** Styling tags are a runtime convention, not a compiler-enforced syntax. The compiler does not validate tag
> names or nesting — it simply passes the text through. Localization tools should preserve tags in translations.

### 13.9 Dialogue Line Semantics

Dialogue text lines (unquoted text after a speaker, or continuation lines) are **implicitly formattable**. Interpolation
with `{expr}` is always available without a `$` prefix — the `dlg` context provides this automatically.

```
dlg greet(playerName: string) {
    @Narrator Hello, {playerName}. You have {getGold()} gold.
}
```

To include a literal `{` or `}` in dialogue text, double it:

```
dlg explain {
    @Narrator Use $"{{expression}}" for interpolation in code.
}
```

The escape sequences from basic strings (Section 4.4.1) are also recognized in dialogue text. Line continuation with `\`
at EOL joins lines with a single space, trimming leading whitespace on the continued line.

---

