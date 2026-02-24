# Writ Language Specification
## 21. Scoping Rules

Writ uses lexical (static) scoping. Every `{ }` block introduces a new scope. Variables are visible from their
declaration point to the end of their enclosing block.

### 21.1 Scope Hierarchy

```
// Global scope — top-level declarations
namespace game;
const MAX_LEVEL: int = 50;
global mut playerGold: int = 0;

// Declaration scope — fn, dlg, entity, impl bodies
fn example(param: int) {          // param is in function scope
    let outer = 10;               // outer is in function scope

    if true {                     // new block scope
        let inner = 20;           // inner is in block scope
        let outer = 30;           // shadows outer (new binding)
    }                             // inner is no longer accessible

    // outer is still 10 (shadow was in inner scope)
}
```

### 21.2 Scope Rules

1. **Block scoping:** Every `{ }` creates a new scope. Variables declared inside are not visible outside.
2. **Shadowing:** A `let` declaration can shadow an outer variable of the same name. The outer binding is restored at
   end of scope. Shadows can have different types.
3. **No hoisting:** Variables are only visible after their declaration point. Forward references to variables are
   compile errors.
4. **Closure capture:** Lambdas and inline functions capture variables from enclosing scopes by reference (for
   `let mut`) or by value (for `let`).
5. **Namespace scoping:** Top-level declarations are in their declared namespace. Non-`pub` declarations are file-local.
   `pub` declarations are accessible from other files and namespaces via `::` or `using` (see Section 23).
6. **`using` scoping:** A `using` declaration is scoped to its enclosing context — file-level `using` is visible
   throughout the file; `using` inside a namespace block is visible only within that block and its nested blocks.

### 21.3 Dialogue Scope

In `dlg` blocks, the entire block is a single scope. Variables declared via `$` escapes are visible for the remainder of
the `dlg` block (including in subsequent dialogue lines for interpolation). Choice branches create nested scopes.

```
dlg example {
    $ let name = getPlayerName();    // visible for rest of dlg
    @Narrator Hello, {name}.
    $ choice {
        "Option A" {
            $ let bonus = 10;        // only visible in this branch
            @Narrator You get {bonus} gold.
        }
        "Option B" {
            // bonus is NOT visible here
            @Narrator Better luck next time.
        }
    }
    @Narrator Goodbye, {name}.       // name still visible
}
```

---

