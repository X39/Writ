# 1. Writ Language Specification
## 12. Functions (fn)

Functions are the primary code construct. They follow C-style syntax with explicit type annotations.

```
fn calculateDamage(base: int, modifier: float, crit: bool) -> int {
    let damage = base * modifier;
    if crit {
        return damage * 2.0;
    }
    damage
}
```

### 12.1 Return Semantics

The last expression in a block is its return value. If the last item in a block is a statement (terminated by `;`), the
block evaluates to void. Explicit `return` is also available for early exits.

```
// Implicit return — last expression is the return value
fn add(a: int, b: int) -> int {
    a + b
}

// Explicit return — for early exit
fn clamp(value: int, max: int) -> int {
    if value > max {
        return max;
    }
    value
}

// Void function — no return type (or explicitly -> void)
fn logDamage(amount: int) {
    log($"Took {amount} damage");
}

// Explicit void annotation (optional, equivalent to omitting -> type)
fn logDamage(amount: int) -> void {
    log($"Took {amount} damage");
}

// Early exit from void function
fn maybeLog(amount: int, verbose: bool) {
    if !verbose {
        return;
    }
    log($"Took {amount} damage");
}
```

The rule applies uniformly to all blocks: function bodies, `if`/`else` branches, `match` arms, and lambda bodies. A
block's value is always the last expression. If the block ends with a `;`, it evaluates to void.

> **Semicolons and block-bodied constructs:** Block-bodied constructs (`if`, `match`, `for`, `while`) are
> terminated by their closing `}` when used as statements. `if` and `match` are expressions — they can appear
> anywhere an expression is valid, including the right-hand side of `let`. `for` and `while` are statements.
> In `let x = if ... { } else { };`, the `;` terminates the `let` statement, not the `if` expression.

### 12.2 Expressions and Blocks

#### 12.2.1 if / else

`if`/`else` is an expression. Each branch is a block, and the block's last expression is the branch's value. When used
as an expression, the `else` branch is required (otherwise the type of the non-taken path is ambiguous).

```
// As expression — returns a value
let msg = if health > 50 {
    "Healthy"
} else {
    "Wounded"
};

// Nested — inner if/else value flows to outer
let tier = if score > 90 {
    "S"
} else if score > 70 {
    "A"
} else {
    "B"
};

// As statement — no value, branches can be void
if damaged {
    playSound("hit");
}
```

#### 12.2.2 match

`match` is an expression. It is exhaustive for enums — the compiler enforces that all variants are handled. Each arm's
block evaluates to a value. When used as an expression, all arms must evaluate to the same type.

```
// As expression — each arm returns a value
let msg = match status {
    QuestStatus::NotStarted => { "Not yet begun" }
    QuestStatus::InProgress(step) => { $"On step {step}" }
    QuestStatus::Completed => { "Done!" }
    QuestStatus::Failed(reason) => { "Failed: " + reason }
};

// Nested — inner match value flows outward
let reward = match difficulty {
    Difficulty::Easy => {
        match playerLevel {
            1 => { 10 }
            2 => { 20 }
        }
    }
    Difficulty::Hard => { 100 }
};

// As statement — arms perform side effects
match event {
    Event::Click(pos) => { handleClick(pos); }
    Event::Key(code) => { handleKey(code); }
}
```

#### 12.2.3 for Loops

`for` iterates over any type that implements `Iterable<T>` (see [Section 10.3](#103-iterablet--for-loop-support)). The
loop variable is immutable by default. Arrays, ranges, and user-defined types that implement `Iterable<T>` are all
supported.

```
for item in inventory {
    log(item.name);
}

for i in 0..10 {
    log($"Step {i}");
}

for member in party.members {
    if let Option::Some(hp) = member[Health] {
        hp.current = min(hp.current + 10, hp.max);
    }
}
```

#### 12.2.4 while Loops

```
while enemy[Health]!.current > 0 {
    attack(enemy);
}
```

#### 12.2.5 break and continue

`break` exits the innermost enclosing loop. `continue` skips to the next iteration of the innermost enclosing loop.
Neither carries a value. There are no labeled loops.

```
for item in inventory {
    if item.name == "Key" {
        useKey(item);
        break;
    }
}

for member in party.members {
    if let Option::Some(hp) = member[Health] {
        if hp.current <= 0 {
            continue;
        }
        hp.current = min(hp.current + 10, hp.max);
    }
}
```

### 12.3 Function Overloading

Functions can be overloaded — multiple functions may share the same name if they have different parameter signatures.
The compiler resolves calls based on argument types at the call site.

```
fn damage(target: Entity, amount: int) {
    target[Health]!.current -= amount;
}

fn damage(target: Entity, amount: int, type: DamageType) {
    let modified = applyResistance(amount, target, type);
    target[Health]!.current -= modified;
}

// Resolved by argument count / types
damage(enemy, 10);
damage(enemy, 10, DamageType::Fire);
```

Overload resolution rules:

1. The compiler finds all functions with the matching name visible in the current scope.
2. It filters to those whose parameter count and types match the arguments at the call site.
3. If exactly one candidate remains, it is selected.
4. If zero candidates match, it is a compile error.
5. If multiple candidates match (ambiguity), it is a compile error.

> **Note:** Return type alone does not distinguish overloads. Two functions with identical parameter signatures but
> different return types are a compile error.

### 12.4 Lambdas (Anonymous Functions)

Anonymous functions use the `fn` keyword without a name. Parameter types and return type are inferred from context when
omitted. Lambda bodies follow the same return rules as named functions — the last expression is the return value.

```
// Minimal — types inferred from context
let sorted = items.sort(fn(a, b) { a.gold > b.gold });

// Explicit parameter types and return type
let compare = fn(a: int, b: int) -> bool { a > b };

// Multi-statement body — last expression is the return value
let transform = fn(x: int) -> int {
    let doubled = x * 2;
    doubled + 1
};

// Early return with explicit return keyword
let clamp = fn(value: int, max: int) -> int {
    if value > max {
        return max;
    }
    value
};
```

#### 12.4.1 Disambiguation

The parser distinguishes lambdas from named function declarations by the token following `fn`:

- `fn` followed by `(` — lambda (anonymous function expression).
- `fn` followed by IDENT — named function declaration.

This requires one token of lookahead.

#### 12.4.2 Type Inference

When a lambda is used in a context with a known expected type (function parameter, typed variable, contract method), the
compiler infers parameter types and return type from that context. When there is no inference context, all parameter
types and the return type must be explicitly annotated.

```
// Inference from function parameter type
fn applyToAll(items: List<int>, transform: fn(int) -> int) { ... }
applyToAll(scores, fn(x) { x * 2 });    // int inferred from parameter type

// Inference from variable type annotation
let f: fn(int) -> bool = fn(x) { x > 10 };

// No inference context — types required
let f = fn(x: int) -> bool { x > 10 };
```

#### 12.4.3 Function Types

Function types are written as `fn(ParamTypes) -> ReturnType`. Functions that return nothing omit the return type.

```
let predicate: fn(int) -> bool = fn(x) { x > 0 };
let action: fn(Entity) = fn(e) { Entity.destroy(e); };
let combine: fn(int, int) -> int = fn(a, b) { a + b };
```

#### 12.4.4 Capture Semantics

Lambdas capture variables from enclosing scopes. Immutable bindings (`let`) are captured by value. Mutable bindings (
`let mut`) are captured by reference.

```
let bonus = 10;                  // captured by value
let mut count = 0;               // captured by reference

let process = fn(x: int) -> int {
    count += 1;                  // mutates the outer count
    x + bonus
};
```

### 12.5 Methods and the `self` Parameter

Methods declared inside `impl` blocks, entity bodies, or component bodies take an explicit `self` or `mut self`
parameter as their first argument. The `self` keyword refers to the instance the method is called on.

#### 12.5.1 Immutable and Mutable Receivers

- `self` — immutable receiver. The method can read fields and call other `self` methods, but cannot modify fields
  or call `mut self` methods through `self`.
- `mut self` — mutable receiver. The method can read and modify fields, and call any method through `self`.

```
fn greet(self) -> string {
    $"Welcome! I am {self.name}"     // OK — reading a field
}

fn damage(mut self, amount: int) {
    self.current -= amount;          // OK — mut self allows field writes
}
```

The caller's binding must be mutable to call a `mut self` method:

```
let guard = new Guard {};
guard.greet();          // OK — greet takes self (immutable)
guard.damage(10);       // ERROR — damage takes mut self, but guard is not mut

let mut guard2 = new Guard {};
guard2.damage(10);      // OK — guard2 is mutable
```

#### 12.5.2 Static Functions

Functions in `impl` blocks that do not take `self` are static functions. They are called on the type, not on an
instance:

```
impl Merchant {
    fn create(name: string) -> Merchant {
        new Merchant { name: name, gold: 0, reputation: 0.8 }
    }
}

let m = Merchant::create("Tim");
```

#### 12.5.3 Operators

Operator declarations use the `operator` keyword and have an implicit `self` receiver — the left operand (or sole
operand for unary operators). Mutability is determined by the operator kind:

- Read operators (`+`, `-`, `*`, `/`, `%`, `==`, `<`, `[]`, unary `-`, `!`): implicit immutable `self`.
- Write operators (`[]=`): implicit mutable `self`.

```
impl vec2 {
    operator +(other: vec2) -> vec2 {          // implicit self (immutable)
        vec2(self.x + other.x, self.y + other.y)
    }

    operator []=(index: int, value: float) {   // implicit mut self
        match index {
            0 => { self.x = value; }
            1 => { self.y = value; }
        }
    }
}
```

#### 12.5.4 Lifecycle Hooks

Lifecycle hooks use the `on` keyword and have an implicit `mut self` receiver. They do not use explicit `self` in their
syntax because their signature is fixed by the runtime.

**Universal hooks** (available on structs and entities):

| Hook             | Purpose                                                   |
|------------------|-----------------------------------------------------------|
| `on create`      | Post-initialization logic (runs after all fields are set) |
| `on finalize`    | GC cleanup (non-deterministic timing)                     |
| `on serialize`   | Prepare for serialization (park native state)             |
| `on deserialize` | Restore after deserialization (recreate native state)     |

**Entity-specific hooks:**

| Hook                       | Purpose                                                       |
|----------------------------|---------------------------------------------------------------|
| `on destroy`               | Deterministic cleanup when `Entity.destroy(entity)` is called |
| `on interact(who: Entity)` | Host-triggered interaction event                              |

```
on create {
    log($"Spawned: {self.name}");   // self is implicitly available and mutable
}
```

---

