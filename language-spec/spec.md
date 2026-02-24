# 1. Writ Language Specification

**Draft v0.3** — February 2026

---

A game scripting language with first-class dialogue support, C-style scripting,
a Rust-inspired type system, and an entity-component architecture.

**File extension:** `.writ`

---

## Table of Contents

<!-- TOC -->
* [1. Writ Language Specification](#1-writ-language-specification)
  * [Table of Contents](#table-of-contents)
  * [1. Overview & Design Philosophy](#1-overview--design-philosophy)
    * [1.1 Design Goals](#11-design-goals)
    * [1.2 Construct Hierarchy](#12-construct-hierarchy)
  * [2. Project Configuration (writ.toml)](#2-project-configuration-writtoml)
    * [2.1 Format](#21-format)
    * [2.2 Required Fields](#22-required-fields)
    * [2.3 Optional Fields](#23-optional-fields)
    * [2.4 Library Resolution](#24-library-resolution)
    * [2.5 Conditions](#25-conditions)
    * [2.6 Locale Identifiers](#26-locale-identifiers)
  * [3. Naming Conventions & Style Guide](#3-naming-conventions--style-guide)
  * [4. Lexical Structure](#4-lexical-structure)
    * [4.1 Keywords](#41-keywords)
    * [4.2 Sigils & Delimiters](#42-sigils--delimiters)
    * [4.3 Comments](#43-comments)
    * [4.4 String Literals](#44-string-literals)
      * [4.4.1 Basic Strings](#441-basic-strings)
      * [4.4.2 Formattable Strings](#442-formattable-strings)
      * [4.4.3 Raw Strings](#443-raw-strings)
      * [4.4.4 Formattable Raw Strings](#444-formattable-raw-strings)
      * [4.4.5 Dialogue Lines](#445-dialogue-lines)
      * [4.4.6 Runtime Type](#446-runtime-type)
  * [5. Type System](#5-type-system)
    * [5.1 Type Categories](#51-type-categories)
    * [5.2 Type Inference](#52-type-inference)
  * [6. Primitive Types](#6-primitive-types)
    * [6.1 Arrays](#61-arrays)
    * [6.2 Array Literals](#62-array-literals)
    * [6.3 Array Operations](#63-array-operations)
    * [6.4 Array Indexing](#64-array-indexing)
    * [6.5 Parser Disambiguation](#65-parser-disambiguation)
    * [6.6 Ranges](#66-ranges)
    * [6.7 From-End Indexing with ^](#67-from-end-indexing-with-)
    * [6.8 Range in For Loops](#68-range-in-for-loops)
    * [6.9 Range Indexing Contract](#69-range-indexing-contract)
  * [7. Variables & Constants](#7-variables--constants)
    * [7.1 Variable Declarations](#71-variable-declarations)
    * [7.2 Shadowing](#72-shadowing)
    * [7.3 Constants](#73-constants)
  * [8. Structs](#8-structs)
    * [8.1 Construction](#81-construction)
    * [8.2 Lifecycle Hooks](#82-lifecycle-hooks)
    * [8.3 Construction Sequence](#83-construction-sequence)
  * [9. Enums](#9-enums)
    * [9.1 Builtin Enums](#91-builtin-enums)
    * [9.2 if let (Optional Pattern Matching)](#92-if-let-optional-pattern-matching)
    * [9.3 Patterns](#93-patterns)
      * [9.3.1 Literal Patterns](#931-literal-patterns)
      * [9.3.2 Wildcard](#932-wildcard)
      * [9.3.3 Variable Binding](#933-variable-binding)
      * [9.3.4 Enum Destructuring](#934-enum-destructuring)
      * [9.3.5 Nested Destructuring](#935-nested-destructuring)
      * [9.3.6 Or-Patterns](#936-or-patterns)
      * [9.3.7 Range Patterns](#937-range-patterns)
  * [10. Contracts](#10-contracts)
    * [10.1 Builtin Contracts](#101-builtin-contracts)
    * [10.2 Into\<T\> — Type Conversion](#102-intot--type-conversion)
    * [10.3 Iterable\<T\> — For Loop Support](#103-iterablet--for-loop-support)
  * [11. Generics](#11-generics)
    * [11.1 Compiler and Runtime Notes](#111-compiler-and-runtime-notes)
    * [11.2 Generic Call Syntax](#112-generic-call-syntax)
  * [12. Functions (fn)](#12-functions-fn)
    * [12.1 Return Semantics](#121-return-semantics)
    * [12.2 Expressions and Blocks](#122-expressions-and-blocks)
      * [12.2.1 if / else](#1221-if--else)
      * [12.2.2 match](#1222-match)
      * [12.2.3 for Loops](#1223-for-loops)
      * [12.2.4 while Loops](#1224-while-loops)
      * [12.2.5 break and continue](#1225-break-and-continue)
    * [12.3 Function Overloading](#123-function-overloading)
    * [12.4 Lambdas (Anonymous Functions)](#124-lambdas-anonymous-functions)
      * [12.4.1 Disambiguation](#1241-disambiguation)
      * [12.4.2 Type Inference](#1242-type-inference)
      * [12.4.3 Function Types](#1243-function-types)
      * [12.4.4 Capture Semantics](#1244-capture-semantics)
    * [12.5 Methods and the `self` Parameter](#125-methods-and-the-self-parameter)
      * [12.5.1 Immutable and Mutable Receivers](#1251-immutable-and-mutable-receivers)
      * [12.5.2 Static Functions](#1252-static-functions)
      * [12.5.3 Operators](#1253-operators)
      * [12.5.4 Lifecycle Hooks](#1254-lifecycle-hooks)
  * [13. Dialogue Blocks (dlg)](#13-dialogue-blocks-dlg)
    * [13.1 Basic Syntax](#131-basic-syntax)
    * [13.2 Speaker Attribution](#132-speaker-attribution)
    * [13.3 The $ Sigil in Dialogue](#133-the--sigil-in-dialogue)
    * [13.4 Choices](#134-choices)
    * [13.5 Conditional Dialogue](#135-conditional-dialogue)
    * [13.6 Dialogue Transitions](#136-dialogue-transitions)
    * [13.7 Localization Keys](#137-localization-keys)
    * [13.8 Text Styling](#138-text-styling)
    * [13.9 Dialogue Suspension](#139-dialogue-suspension)
    * [13.10 Dialogue Line Semantics](#1310-dialogue-line-semantics)
  * [14. Entities](#14-entities)
    * [14.1 Entity Declaration](#141-entity-declaration)
    * [14.2 Creating Entities](#142-creating-entities)
    * [14.3 Component Access](#143-component-access)
    * [14.4 Singleton Entities](#144-singleton-entities)
    * [14.5 Entity References & EntityList](#145-entity-references--entitylist)
    * [14.5.1 Entity Handles](#1451-entity-handles)
    * [14.5.2 Entity Static Methods](#1452-entity-static-methods)
    * [14.6 Lifecycle Hooks](#146-lifecycle-hooks)
      * [14.6.1 Universal Hooks](#1461-universal-hooks)
      * [14.6.2 Entity-Specific Hooks](#1462-entity-specific-hooks)
    * [14.7 Entity Lowering](#147-entity-lowering)
      * [14.7.1 TypeDef Generation](#1471-typedef-generation)
      * [14.7.2 Method Lowering](#1472-method-lowering)
      * [14.7.3 Lifecycle Hook Lowering](#1473-lifecycle-hook-lowering)
      * [14.7.4 Component Access Lowering](#1474-component-access-lowering)
      * [14.7.5 Construction Sequence](#1475-construction-sequence)
  * [15. Components](#15-components)
    * [15.1 Component Declarations](#151-component-declarations)
    * [15.2 Component Access](#152-component-access)
    * [15.3 Runtime Behavior](#153-runtime-behavior)
  * [16. Attributes](#16-attributes)
    * [16.1 Syntax](#161-syntax)
    * [16.2 Builtin Attributes](#162-builtin-attributes)
    * [16.3 Parser Disambiguation](#163-parser-disambiguation)
    * [16.4 Conditional Compilation](#164-conditional-compilation)
  * [17. Operators & Overloading](#17-operators--overloading)
    * [17.1 Operator Precedence (highest to lowest)](#171-operator-precedence-highest-to-lowest)
    * [17.2 Overloading Syntax](#172-overloading-syntax)
    * [17.3 Compound Assignment](#173-compound-assignment)
    * [17.4 Derived Operators](#174-derived-operators)
  * [18. Error Handling](#18-error-handling)
    * [18.1 The ? Operator (Null Propagation)](#181-the--operator-null-propagation)
    * [18.2 The ! Operator (Unwrap or Crash)](#182-the--operator-unwrap-or-crash)
    * [18.3 The try Keyword (Error Propagation)](#183-the-try-keyword-error-propagation)
    * [18.4 The Error Contract](#184-the-error-contract)
    * [18.5 Separation Summary](#185-separation-summary)
  * [19. Nullability & Optionals](#19-nullability--optionals)
  * [20. Concurrency](#20-concurrency)
    * [20.1 Execution Model](#201-execution-model)
    * [20.2 Concurrency Primitives](#202-concurrency-primitives)
    * [20.3 Task Lifetime Rules](#203-task-lifetime-rules)
  * [21. Scoping Rules](#21-scoping-rules)
    * [21.1 Scope Hierarchy](#211-scope-hierarchy)
    * [21.2 Scope Rules](#212-scope-rules)
    * [21.3 Dialogue Scope](#213-dialogue-scope)
  * [22. Globals & Atomic Access](#22-globals--atomic-access)
    * [22.1 Global Variables](#221-global-variables)
    * [22.2 Concurrency Safety](#222-concurrency-safety)
    * [22.3 Atomic Blocks](#223-atomic-blocks)
  * [23. Modules & Namespaces](#23-modules--namespaces)
    * [23.1 Declarative Namespace](#231-declarative-namespace)
    * [23.2 Block Namespace](#232-block-namespace)
    * [23.3 Root Namespace](#233-root-namespace)
    * [23.4 `using` Declarations](#234-using-declarations)
      * [23.4.1 Alias Form](#2341-alias-form)
      * [23.4.2 Placement Rules](#2342-placement-rules)
      * [23.4.3 Scope of `using`](#2343-scope-of-using)
    * [23.5 Same-Namespace Visibility](#235-same-namespace-visibility)
    * [23.6 Visibility Modifiers](#236-visibility-modifiers)
      * [23.6.1 Top-Level Declarations](#2361-top-level-declarations)
      * [23.6.2 Type Members](#2362-type-members)
      * [23.6.3 Entity and Component Members](#2363-entity-and-component-members)
      * [23.6.4 Contracts and Implementations](#2364-contracts-and-implementations)
      * [23.6.5 Enum Variants](#2365-enum-variants)
      * [23.6.6 Visibility Summary](#2366-visibility-summary)
    * [23.7 Name Conflicts](#237-name-conflicts)
    * [23.8 Cross-Namespace Access](#238-cross-namespace-access)
    * [23.9 Root Namespace Prefix (`::`)](#239-root-namespace-prefix-)
    * [23.10 `::` Resolution](#2310--resolution)
    * [23.11 File Path Convention](#2311-file-path-convention)
  * [24. External Declarations](#24-external-declarations)
    * [24.1 Runtime-Provided Externals](#241-runtime-provided-externals)
    * [24.2 Library Imports](#242-library-imports)
      * [24.2.1 Import Attribute Parameters](#2421-import-attribute-parameters)
      * [24.2.2 Examples](#2422-examples)
    * [24.3 Architecture Identifiers](#243-architecture-identifiers)
    * [24.4 Library Resolution](#244-library-resolution)
    * [24.5 Symbol Resolution](#245-symbol-resolution)
    * [24.6 Crash Semantics](#246-crash-semantics)
  * [25. Localization](#25-localization)
    * [25.1 Overview](#251-overview)
    * [25.2 String Extraction & the Localization Key](#252-string-extraction--the-localization-key)
      * [25.2.1 Key Computation](#2521-key-computation)
      * [25.2.2 FNV-1a 32-bit Algorithm](#2522-fnv-1a-32-bit-algorithm)
      * [25.2.3 Deduplication Index](#2523-deduplication-index)
    * [25.3 CSV Localization Format](#253-csv-localization-format)
      * [25.3.1 CSV Encoding Rules](#2531-csv-encoding-rules)
      * [25.3.2 Column Structure](#2532-column-structure)
      * [25.3.3 Example](#2533-example)
      * [25.3.4 Interpolation Slot Validation](#2534-interpolation-slot-validation)
    * [25.4 Compiler Tooling](#254-compiler-tooling)
      * [25.4.1 Incremental Export](#2541-incremental-export)
    * [25.5 Runtime String Table Lookup](#255-runtime-string-table-lookup)
    * [25.6 Structural Overrides with [Locale]](#256-structural-overrides-with-locale)
      * [25.6.1 Syntax](#2561-syntax)
      * [25.6.2 Rules](#2562-rules)
      * [25.6.3 Runtime Dispatch](#2563-runtime-dispatch)
    * [25.7 Design Rationale](#257-design-rationale)
  * [26. Standard Library Builtins](#26-standard-library-builtins)
    * [26.1 Compiler-Known Types](#261-compiler-known-types)
    * [26.2 Compiler-Known Contracts](#262-compiler-known-contracts)
    * [26.3 Standard Library Types](#263-standard-library-types)
  * [27. Grammar Summary (EBNF)](#27-grammar-summary-ebnf)
  * [28. Lowering Reference](#28-lowering-reference)
    * [28.1 Dialogue Lowering](#281-dialogue-lowering)
    * [28.2 Full Dialogue Lowering Example](#282-full-dialogue-lowering-example)
    * [28.3 Entity Lowering](#283-entity-lowering)
    * [28.4 Localized Dialogue Lowering](#284-localized-dialogue-lowering)
    * [28.5 Runtime Functions](#285-runtime-functions)
* [Writ IL Specification](#writ-il-specification)
  * [2.1 Register-Based Virtual Machine](#21-register-based-virtual-machine)
  * [2.2 Typed IL with Generic Preservation](#22-typed-il-with-generic-preservation)
  * [2.3 Execution Model: Cooperative Yielding with Preemptive Serialization](#23-execution-model-cooperative-yielding-with-preemptive-serialization)
  * [2.4 Binary Format](#24-binary-format)
  * [2.5 Instruction Encoding](#25-instruction-encoding)
  * [2.6 Calling Convention](#26-calling-convention)
  * [2.7 Operator Dispatch](#27-operator-dispatch)
  * [2.8 Serialization Critical Sections — REMOVED](#28-serialization-critical-sections--removed)
  * [2.9 Memory Model](#29-memory-model)
    * [2.9.1 Value Types vs Reference Types](#291-value-types-vs-reference-types)
    * [2.9.2 Assignment and Mutability](#292-assignment-and-mutability)
    * [2.9.3 Closure Captures](#293-closure-captures)
    * [2.9.4 String Handling](#294-string-handling)
    * [2.9.5 Entity Lifecycle](#295-entity-lifecycle)
    * [2.9.6 GC Roots](#296-gc-roots)
    * [2.9.7 Garbage Collection](#297-garbage-collection)
    * [2.9.8 IL Implications](#298-il-implications)
  * [2.10 Self Parameter](#210-self-parameter)
  * [2.11 Construction Model](#211-construction-model)
  * [2.12 Delegate Model (Closures & Function Values)](#212-delegate-model-closures--function-values)
    * [2.12.1 Delegate Structure](#2121-delegate-structure)
    * [2.12.2 Creation Scenarios](#2122-creation-scenarios)
    * [2.12.3 Invocation](#2123-invocation)
    * [2.12.4 Virtual Method References](#2124-virtual-method-references)
    * [2.12.5 Relationship to Function Types](#2125-relationship-to-function-types)
  * [2.13 Save/Load Serialization](#213-saveload-serialization)
    * [2.13.1 Spec Requirements](#2131-spec-requirements)
    * [2.13.2 Module Versioning](#2132-module-versioning)
    * [2.13.3 Extern Calls During Serialization](#2133-extern-calls-during-serialization)
  * [2.14 Runtime-Host Interface](#214-runtime-host-interface)
    * [2.14.1 Architecture](#2141-architecture)
    * [2.14.2 Runtime -> Host (requests — runtime suspends until host confirms)](#2142-runtime---host-requests--runtime-suspends-until-host-confirms)
    * [2.14.3 Host -> Runtime (commands the host sends)](#2143-host---runtime-commands-the-host-sends)
    * [2.14.4 Entity Ownership Model](#2144-entity-ownership-model)
    * [2.14.5 Singleton Entities and the Host](#2145-singleton-entities-and-the-host)
    * [2.14.6 Scripted Entities: Runtime Requirements](#2146-scripted-entities-runtime-requirements)
    * [2.14.7 Runtime Logging Interface](#2147-runtime-logging-interface)
    * [2.14.8 Implementation Guidance](#2148-implementation-guidance)
  * [2.15 IL Type System](#215-il-type-system)
    * [2.15.1 Register Model](#2151-register-model)
    * [2.15.2 Primitive Type Tags](#2152-primitive-type-tags)
    * [2.15.3 Type Reference Encoding](#2153-type-reference-encoding)
    * [2.15.4 Generic Representation](#2154-generic-representation)
    * [2.15.5 Enum Representation](#2155-enum-representation)
  * [2.16 IL Module Format](#216-il-module-format)
    * [2.16.1 Binary Container](#2161-binary-container)
    * [2.16.2 Multi-Module Architecture](#2162-multi-module-architecture)
    * [2.16.3 Module Versioning](#2163-module-versioning)
    * [2.16.4 Metadata Tokens](#2164-metadata-tokens)
    * [2.16.5 Metadata Tables](#2165-metadata-tables)
    * [2.16.6 Method Body Layout](#2166-method-body-layout)
    * [2.16.7 Entity Construction Buffering](#2167-entity-construction-buffering)
    * [2.16.8 The `writ-runtime` Module](#2168-the-writ-runtime-module)
  * [2.17 Execution Model](#217-execution-model)
    * [2.17.1 Call Stack](#2171-call-stack)
    * [2.17.2 Task States](#2172-task-states)
    * [2.17.3 Transition Points](#2173-transition-points)
    * [2.17.4 Entry Points](#2174-entry-points)
    * [2.17.5 Scheduling and Execution Limits](#2175-scheduling-and-execution-limits)
    * [2.17.6 Atomic Sections](#2176-atomic-sections)
    * [2.17.7 Crash Propagation and Defer Unwinding](#2177-crash-propagation-and-defer-unwinding)
    * [2.17.8 Task Tree](#2178-task-tree)
  * [2.18 `writ-runtime` Module Contents](#218-writ-runtime-module-contents)
    * [2.18.1 Core Enums](#2181-core-enums)
      * [Option\<T\>](#optiont)
      * [Result\<T, E: Error\>](#resultt-e-error)
    * [2.18.2 Range\<T\>](#2182-ranget)
    * [2.18.3 Contracts](#2183-contracts)
    * [2.18.4 Primitive Pseudo-Types](#2184-primitive-pseudo-types)
    * [2.18.5 Primitive Contract Implementations](#2185-primitive-contract-implementations)
    * [2.18.6 Array Type](#2186-array-type)
    * [2.18.7 Entity Base Type](#2187-entity-base-type)
    * [2.18.8 Versioning](#2188-versioning)
  * [3.0 Meta](#30-meta)
  * [3.1 Data Movement](#31-data-movement)
  * [3.2 Integer Arithmetic](#32-integer-arithmetic)
  * [3.3 Float Arithmetic](#33-float-arithmetic)
  * [3.4 Bitwise & Logical](#34-bitwise--logical)
  * [3.5 Comparison](#35-comparison)
  * [3.6 Control Flow](#36-control-flow)
  * [3.7 Calls](#37-calls)
  * [3.8 Object Model](#38-object-model)
  * [3.9 Arrays](#39-arrays)
  * [3.10 Type Operations](#310-type-operations)
  * [3.11 Concurrency](#311-concurrency)
  * [3.12 Globals & Atomics](#312-globals--atomics)
  * [3.13 Conversion](#313-conversion)
  * [3.14 Strings](#314-strings)
  * [3.15 Boxing](#315-boxing)
  * [3.16 Serialization Control — REMOVED](#316-serialization-control--removed)
  * [4.0 Instruction Count by Category](#40-instruction-count-by-category)
  * [4.1 Instruction Shape Reference](#41-instruction-shape-reference)
  * [4.2 Opcode Assignment Table](#42-opcode-assignment-table)
    * [0x00 — Meta](#0x00--meta)
    * [0x01 — Data Movement](#0x01--data-movement)
    * [0x02 — Integer Arithmetic](#0x02--integer-arithmetic)
    * [0x03 — Float Arithmetic](#0x03--float-arithmetic)
    * [0x04 — Bitwise & Logical](#0x04--bitwise--logical)
    * [0x05 — Comparison](#0x05--comparison)
    * [0x06 — Control Flow](#0x06--control-flow)
    * [0x07 — Calls & Delegates](#0x07--calls--delegates)
    * [0x08 — Object Model](#0x08--object-model)
    * [0x09 — Arrays](#0x09--arrays)
    * [0x0A — Type Operations](#0x0a--type-operations)
    * [0x0B — Concurrency](#0x0b--concurrency)
    * [0x0C — Globals & Atomics](#0x0c--globals--atomics)
    * [0x0D — Conversion](#0x0d--conversion)
    * [0x0E — Strings](#0x0e--strings)
    * [0x0F — Boxing](#0x0f--boxing)
* [Appendix](#appendix)
  * [A. Open Questions](#a-open-questions)
  * [B. IL Decision Log](#b-il-decision-log)
<!-- TOC -->

---

## 1. Overview & Design Philosophy

Writ is a statically-typed game scripting language designed around two core constructs: `fn` for general-purpose C-style
logic, and `dlg` for dialogue authoring. The language also provides `entity` and `component` declarations for defining
game objects within an entity-component architecture. All higher-level constructs (`dlg`, `entity`) lower to simpler
primitives at compile time.

### 1.1 Design Goals

- **Trivial tokenization.** The lexer should be able to classify any token with minimal lookahead (ideally one token).
  Sigils and keywords create unambiguous mode switches.
- **Dialogue-first ergonomics.** Writing dialogue lines should require minimal ceremony. Non-developers should be able
  to author `dlg` blocks without understanding the type system.
- **C-style scripting familiarity.** The `fn` world uses braces, semicolons, and standard operators. Anyone who has
  written C, Java, JavaScript, or Rust will feel at home.
- **Rust-inspired type safety.** Contracts (traits), bounded generics, tagged enums, Result/Option types, and
  immutability by default catch errors at compile time.
- **Entity-component architecture.** Game objects are declared as entities with composable components, providing a
  familiar and flexible game development model.
- **Runtime agnosticism.** The language compiles to an intermediate representation. Runtimes can interpret it directly
  or JIT-compile hot paths.

### 1.2 Construct Hierarchy

All higher-level constructs lower to simpler ones at compile time:

```
dlg      →  fn calls (say, choice, etc.)
entity   →  struct + impl + component registration
contract →  dispatch table entries
operator →  contract impl (Add, Sub, etc.)
T?       →  Option<T>
null     →  Option::None
```

> **Note:** The runtime only needs to understand functions, structs, and a dispatch table. Everything else is compiler
> sugar.

---

## 2. Project Configuration (writ.toml)

Every Writ project requires a `writ.toml` file at the project root. This file defines project metadata, compiler
settings, and localization configuration.

### 2.1 Format

The file uses [TOML v1.0](https://toml.io/en/v1.0.0) syntax.

### 2.2 Required Fields

```toml
[project]
name = "my-game"
version = "0.1.0"

[locale]
default = "en"
```

### 2.3 Optional Fields

```toml
[project]
name = "my-game"
version = "0.1.0"
authors = ["Dev Name"]

[locale]
default = "en"
supported = ["en", "de", "fr", "ja", "ko", "zh"]

[compiler]
# Source directories (relative to writ.toml)
sources = ["src/", "dialogue/"]
# Output directory for compiled artifacts
output = "build/"

[locale.export]
# Output directory for localization CSV files
output = "locale/"

# Optional: library name mappings for [Import] attributes.
# Maps logical library names to architecture-specific names.
# These serve as defaults — [Import] attribute overrides take precedence.
[libraries.physics]
default = "libphysics"
x64 = "physics64"
arm64 = "physics_arm"

[libraries.audio]
default = "fmod"

# Optional: named conditions for conditional compilation.
# Can be overridden via compiler flags.
[conditions]
debug = true
```

### 2.4 Library Resolution

The optional `[libraries.<name>]` sections map logical library names (as used in `[Import("name")]` attributes) to
architecture-specific library names. Each entry supports a `default` key and architecture-specific overrides using the
identifiers defined in [Section 24.3](#243-architecture-identifiers).

Resolution precedence for a library name:

1. Architecture-specific override in the `[Import]` attribute itself.
2. Architecture-specific override in `writ.toml` `[libraries.<name>]`.
3. `default` in `writ.toml` `[libraries.<name>]`.
4. The logical name from the `[Import]` positional argument, as-is.

The `[libraries]` section is entirely optional. Projects that specify all overrides in `[Import]` attributes do not need
it. Projects that distribute pre-compiled artifacts without source can rely solely on attribute-level overrides.

### 2.5 Conditions

The optional `[conditions]` section defines named conditions for conditional compilation (
see [Section 16.4](#164-conditional-compilation)). Each key is a condition name and its value is a boolean indicating
whether the condition is active.

```toml
[conditions]
debug = true
playstation = false
xbox = false
editor = true
```

Conditions can also be set or overridden via compiler flags, allowing build scripts to control platform targeting
without modifying `writ.toml`:

```
writc --condition playstation=true --condition debug=false
```

Compiler flags take precedence over `writ.toml` values. A condition referenced in a `[Conditional("name")]` attribute
that is not defined in either `writ.toml` or compiler flags is treated as inactive (false).

### 2.6 Locale Identifiers

Locale identifiers follow [BCP 47](https://www.rfc-editor.org/info/bcp47) language tags. Common examples: `en`, `de`,
`fr`, `ja`, `ko`, `zh`, `pt-BR`, `en-GB`. The `default` locale is the language used for inline dialogue text in `.writ`
source files.

The `supported` array lists all locales the project targets. If omitted, only the `default` locale is assumed. The
`writ loc export` tool uses this list to generate CSV column headers.

---

## 3. Naming Conventions & Style Guide

The following naming conventions are enforced by the compiler as warnings and by the language server as suggestions.

| Construct                                         | Convention           | Examples                                  |
|---------------------------------------------------|----------------------|-------------------------------------------|
| Types (struct, entity, enum, contract, component) | PascalCase           | `Merchant`, `QuestStatus`, `Interactable` |
| Enum variants                                     | PascalCase           | `InProgress`, `Completed`, `None`         |
| Functions, methods                                | camelCase            | `calculateDamage`, `getOrCreate`          |
| Variables, parameters                             | camelCase            | `playerName`, `goldAmount`                |
| Constants                                         | SCREAMING_SNAKE_CASE | `MAX_HEALTH`, `DEFAULT_GOLD`              |
| Namespaces                                        | snake_case           | `survival`, `quest_system`                |
| Attributes                                        | PascalCase           | `Singleton`, `Deprecated`, `Locale`       |
| Builtin primitive types                           | lowercase            | `int`, `float`, `bool`, `string`          |
| Builtin generic types                             | PascalCase           | `Option`, `Result`, `List`, `Map`         |

> **Note:** Primitive types (`int`, `float`, `bool`, `string`) are lowercase because they are language keywords. Array
> types use postfix `[]` notation (e.g., `int[]`). Standard library types (`Option`, `Result`, `List`, `Map`, `Set`,
`EntityList`) follow PascalCase as they are regular types, even though `Option` and `Result` have special compiler
> support.

---

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

## 5. Type System

Writ uses a static type system with type inference for local variables. Types are checked at compile time. The runtime
carries type tags for dynamic dispatch of contract methods.

### 5.1 Type Categories

| Category       | Examples                          | Notes                                                        |
|----------------|-----------------------------------|--------------------------------------------------------------|
| Primitives     | `int`, `float`, `bool`, `string`  | Value types, always non-null, lowercase keywords             |
| Arrays         | `T[]`                             | Fixed-type, growable ordered collections with literal syntax |
| Structs        | `struct Merchant { ... }`         | User-defined composite types                                 |
| Entities       | `entity Guard { ... }`            | Game objects with components and lifecycle                   |
| Components     | `extern component Health { ... }` | Extern data schemas attached to entities via `use`           |
| Enums          | `enum QuestStatus { ... }`        | Tagged unions with variant data                              |
| Ranges         | `Range<T>`                        | Compiler-known interval type, created with `..` and `..=`    |
| Nullable       | `T?`                              | Sugar for `Option<T>`                                        |
| Result         | `Result<T, E>`                    | `E` must implement `Error` contract                          |
| Function types | `fn(int, int) -> int`             | First-class function references                              |
| Generic        | `T`, `T: Contract`                | Bounded or unbounded type parameters                         |

### 5.2 Type Inference

Local variable types are inferred from their initializer. Function signatures, struct fields, and entity properties must
be fully annotated.

```
let x = 42;                // inferred as int
let name = "hello";        // inferred as string
let pos = new vec2 { x: 1.0, y: 2.0 };  // inferred as vec2

// Function signatures require explicit types
fn add(a: int, b: int) -> int {
    a + b
}
```

---

## 6. Primitive Types

| Type     | Description                      | Default Value |
|----------|----------------------------------|---------------|
| `int`    | 64-bit signed integer            | `0`           |
| `float`  | 64-bit IEEE 754 floating point   | `0.0`         |
| `bool`   | Boolean value                    | `false`       |
| `string` | UTF-8 encoded string (immutable) | `""`          |

### 6.1 Arrays

Arrays are ordered, growable, homogeneous collections. They are a compiler-known semi-primitive — not a single machine
word, but with literal syntax and built-in operations. Like `string`, the compiler understands arrays directly; they are
not a standard library type.

The array type is written with postfix `[]` notation: `int[]`, `string[]`, `vec2[]`.

| Type  | Description                         | Default Value | Literal Syntax |
|-------|-------------------------------------|---------------|----------------|
| `T[]` | Ordered, growable collection of `T` | `[]`          | `[expr, ...]`  |

### 6.2 Array Literals

The `[expr, expr, ...]` syntax constructs an array. All elements must be the same type. The element type is inferred
from the contents or from the expected type context.

```
let numbers = [1, 2, 3];              // int[]
let names = ["Alice", "Bob"];          // string[]
let empty: int[] = [];                 // empty array, type from annotation
let mixed = [1, 2.0];                 // COMPILE ERROR: int and float are not the same type
```

An empty literal `[]` requires a type context — either a variable type annotation or an expected parameter type. Without
context, it is a compile error.

```
let items: int[] = [];                 // ok: type from annotation
fn process(items: int[]) { ... }
process([]);                           // ok: type from parameter

let unknown = [];                      // COMPILE ERROR: cannot infer element type
```

### 6.3 Array Operations

The following operations are compiler-known and provided by the runtime. Arrays are mutable — elements can be added and
removed. The array itself must be `let mut` to allow structural mutation (add/remove). Element assignment through
indexing also requires `let mut`.

| Operation              | Signature                 | Description                                                                                |
|------------------------|---------------------------|--------------------------------------------------------------------------------------------|
| `.length`              | `int` (read-only)         | Number of elements.                                                                        |
| `[index]`              | `T`                       | Access element by zero-based index. Out-of-bounds crashes the task (with defer unwinding). |
| `.add(item)`           | `fn(item: T)`             | Append an element to the end.                                                              |
| `.removeAt(index)`     | `fn(index: int)`          | Remove the element at the given index. Out-of-bounds crashes the task.                     |
| `.insert(index, item)` | `fn(index: int, item: T)` | Insert an element at the given index, shifting subsequent elements.                        |
| `.contains(item)`      | `fn(item: T) -> bool`     | Returns `true` if the item is in the array. `T` must implement `Eq`.                       |
| `.iterator()`          | `Iterator<T>`             | Returns an iterator over elements. Arrays implement `Iterable<T>` (see Section 10.3).      |

```
let mut inventory = ["Sword", "Shield"];
inventory.add("Potion");              // ["Sword", "Shield", "Potion"]
inventory.removeAt(0);                // ["Shield", "Potion"]
let count = inventory.length;         // 2
let has = inventory.contains("Shield"); // true

// Immutable array — structure cannot change
let fixed = [1, 2, 3];
fixed.add(4);                         // COMPILE ERROR: fixed is not mutable
let x = fixed[0];                     // ok: reading is always permitted
```

### 6.4 Array Indexing

Array indexing uses the `[]` operator. It returns `T` directly (not `Option<T>`). Out-of-bounds access crashes the
current task with defer unwinding — this matches the crash semantics of `!` and failed library loads.

```
let items = [10, 20, 30];
let first = items[0];                 // 10
let bad = items[99];                  // RUNTIME CRASH: index out of bounds
```

### 6.5 Parser Disambiguation

The `[` token has three roles depending on context:

1. **Array literal** — at the start of an expression (after `=`, as argument, etc.): `[1, 2, 3]`
2. **Index / component access** — as a postfix operator after an expression: `items[0]`, `guard[Health]`
3. **Attribute** — at statement level before a declaration keyword: `[Singleton]`

The parser resolves (1) vs (2) by position: `[` at the start of an expression is a literal, `[` after an expression is
postfix. For (3), see [Section 16.3](#163-parser-disambiguation).

> **Note:** `string` is listed in the primitive types table despite not being a machine word, because it is a language
> keyword. Arrays follow the same pattern — compiler-known, with dedicated syntax, but not a single machine word.
> Standard
> library types like `List<T>` may provide higher-level collection abstractions on top of arrays.

### 6.6 Ranges

`Range<T>` is a compiler-known type representing an interval between two values. It is created with the `..` (exclusive
end) or `..=` (inclusive end) operators.

```
let r = 0..10;        // Range<int>, exclusive: [0, 10)
let ri = 0..=10;      // Range<int>, inclusive: [0, 10]
let pct = 0.0..1.0;   // Range<float>, exclusive
```

Start or end may be omitted when used inside `[]` indexing to mean "from the beginning" or "to the end":

```
let items = [10, 20, 30, 40, 50];
items[1..4]     // [20, 30, 40]
items[..3]      // [10, 20, 30]
items[2..]      // [30, 40, 50]
```

### 6.7 From-End Indexing with ^

Inside `[]` indexing, the `^n` syntax means "n from the end." The compiler desugars `^n` to `collection.length - n` at
the call site. `^` is only valid inside `[]` — it is not a general-purpose operator.

```
let items = [10, 20, 30, 40, 50];
items[^1]       // 50 (last element, desugars to items[items.length - 1])
items[^2]       // 40 (second from end)
items[..^1]     // [10, 20, 30, 40] (everything except last)
items[^3..^1]   // [30, 40] (third from end to second from end)

let text = "Hello, world!";
text[..^1]      // "Hello, world" (drop last char)
text[7..]       // "world!"
```

### 6.8 Range in For Loops

Ranges are iterable. When used with `for`, exclusive ranges (`..`) iterate up to but not including the end, and
inclusive ranges (`..=`) include the end:

```
for i in 0..5 {
    // i = 0, 1, 2, 3, 4
}

for i in 1..=5 {
    // i = 1, 2, 3, 4, 5
}
```

### 6.9 Range Indexing Contract

Types that support range-based slicing implement `Index<Range<int>, R>` where `R` is the return type of the slice.
Arrays and strings have compiler-provided implementations:

| Type     | Index Key    | Returns  | Description      |
|----------|--------------|----------|------------------|
| `T[]`    | `int`        | `T`      | Single element   |
| `T[]`    | `Range<int>` | `T[]`    | Sub-array (copy) |
| `string` | `Range<int>` | `string` | Substring        |

User-defined types may implement `Index<Range<int>, R>` to support range-based slicing via the standard operator
overloading mechanism.

---

## 7. Variables & Constants

### 7.1 Variable Declarations

Variables are declared with `let` (immutable) or `let mut` (mutable). Immutability is the default. Type is inferred from
the initializer or can be explicitly annotated.

```
let name = "Aria";              // immutable, inferred string
let mut health = 100;            // mutable, inferred int
let pos: vec2 = vec2 { x: 0.0, y: 0.0 };  // explicit type annotation

name = "Bob";      // COMPILE ERROR: name is immutable
health += 10;       // ok: health is mutable
```

### 7.2 Shadowing

Variables can be shadowed by a new `let` declaration in the same scope. The new binding can have a different type. This
allows transformations without mutability.

```
let x = 10;
let x = x * 2;      // shadows, x is now 20 (still immutable)
let x = "hello";    // shadows again, different type
```

### 7.3 Constants

Constants are declared with `const` at the top level. They must have a compile-time known value.

```
const MAX_HEALTH: int = 100;
const GAME_TITLE: string = "My RPG";
const DEFAULT_SPEED: float = 5.0;
```

---

## 8. Structs

Structs are named composite types with named fields. They support methods and operator overloading via `impl` blocks,
and lifecycle hooks directly in the struct body.

```
struct Merchant {
    name: string,
    gold: int,
    reputation: float = 0.5,
}

impl Merchant {
    fn greet(self) -> string {
        $"Welcome! I am {self.name}"
    }
}

// Construction uses the `new` keyword with named fields
let m = new Merchant { name: "Old Tim", gold: 100 };
```

### 8.1 Construction

Structs are constructed with the `new` keyword followed by the type name and brace-enclosed field initializers. Fields
with default values may be omitted. Fields without defaults are required at every construction site.

```
let m = new Merchant { name: "Old Tim", gold: 100 };           // reputation defaults to 0.5
let m2 = new Merchant { name: "Sue", gold: 50, reputation: 0.9 };
```

The `new` keyword disambiguates construction from block expressions, making the syntax unambiguous for the parser.

For convenience factories, use static methods in `impl` blocks:

```
impl Merchant {
    fn create(name: string) -> Merchant {
        new Merchant { name: name, gold: 0 }
    }
}

let m = Merchant::create("Tim");
```

### 8.2 Lifecycle Hooks

Structs may define lifecycle hooks using the `on` keyword directly in the struct body. All hooks receive an implicit
`mut self` parameter.

```
struct NativeConnection {
    url: string,
    handle: int = 0,

    on create {
        self.handle = native_connect(self.url);
    }

    on finalize {
        native_disconnect(self.handle);
    }

    on serialize {
        self.handle = 0;
    }

    on deserialize {
        self.handle = native_connect(self.url);
    }
}
```

| Hook             | When                              | Purpose                                                 |
|------------------|-----------------------------------|---------------------------------------------------------|
| `on create`      | After all fields are initialized  | Post-initialization logic                               |
| `on finalize`    | GC is about to collect the object | Last-chance cleanup of native resources                 |
| `on serialize`   | Before the object is serialized   | Park native state (clear handles, prepare for snapshot) |
| `on deserialize` | After the object is deserialized  | Recreate native state from stored fields                |

**Implicit `on create`:** Every struct conceptually has an `on create` hook. The compiler generates code to initialize
all fields (explicit values and defaults) before the user-written `on create` body runs. If no user `on create` is
written, construction simply initializes fields and returns.

**`on finalize` semantics:** The finalizer runs when the garbage collector determines the object is unreachable. Timing
is non-deterministic. Storing `self` in a reachable location during `on finalize` (resurrection) is undefined behavior.

**Hook failure semantics:** If any lifecycle hook crashes (via `!` unwrap, out-of-bounds access, etc.), the crash
unwinds and terminates the calling task's entire call stack. The runtime must log the failure to the host via the
runtime logging interface (see IL spec §1.14.7). Specific consequences by hook:

- `on create` crash: The object is left in a partially initialized state. The crash terminates the task that called
  `new`.
- `on serialize` crash: The runtime logs the error. Whether the save proceeds without this object or fails entirely is
  runtime-defined.
- `on deserialize` crash: The runtime logs the error. The object exists but may have unrecovered native state.
- `on finalize` crash: The runtime logs the error and continues GC collection. The finalizer does not retry.

### 8.3 Construction Sequence

`new Merchant { name: "Tim", gold: 100 }` compiles to the following IL:

```
NEW           r0, Merchant_type       // 1. Allocate zeroed memory
LOAD_FLOAT    r1, 0.5                 // 2. Load default for reputation
SET_FIELD     r0, reputation_field, r1
LOAD_STRING   r1, "Tim"_idx          // 3. Apply construction-site overrides
SET_FIELD     r0, name_field, r1
LOAD_INT      r1, 100
SET_FIELD     r0, gold_field, r1
CALL          r_, Merchant::__on_create, r0  // 4. Run on create (if defined)
```

The full sequence:

1. **NEW** — allocate zeroed memory for the struct.
2. **SET_FIELD** — apply default values for all fields that have them.
3. **SET_FIELD** — apply construction-site overrides (these overwrite defaults where specified).
4. **CALL `__on_create`** — run the user-defined `on create` body, if present. At this point, all fields are fully
   initialized.

---

## 9. Enums

Enums are tagged unions. Each variant can optionally carry named data fields. Pattern matching via `match` is
exhaustive — the compiler enforces that all variants are handled.

```
enum QuestStatus {
    NotStarted,
    InProgress(currentStep: int),
    Completed,
    Failed(reason: string),
}

fn describeQuest(status: QuestStatus) -> string {
    match status {
        QuestStatus::NotStarted => { "Not yet begun." }
        QuestStatus::InProgress(step) => { $"On step {step}" }
        QuestStatus::Completed => { "Done!" }
        QuestStatus::Failed(reason) => { "Failed: " + reason }
    }
}
```

### 9.1 Builtin Enums

The following enums are compiler-known and have special syntax support:

```
// Option<T> — nullable values. T? is sugar for Option<T>.
enum Option<T> {
    None,
    Some(value: T),
}

// Result<T, E> — fallible operations. E must implement Error.
enum Result<T, E: Error> {
    Ok(value: T),
    Err(error: E),
}
```

### 9.2 if let (Optional Pattern Matching)

The `if let` construct provides a concise way to match a single pattern, particularly useful for `Option` and
single-variant checks.

```
if let Option::Some(healer) = party.healer {
    healer.heal(player);
}

// With else branch:
if let Option::Some(quest) = activeQuest {
    showQuestUI(quest);
} else {
    showNoQuestMessage();
}
```

### 9.3 Patterns

Patterns appear in `match` arms and `if let` bindings. Writ supports seven pattern forms.

#### 9.3.1 Literal Patterns

Integer, string, boolean, and null literals match by value.

```
match command {
    42 => { handleSpecial(); }
    "quit" => { exit(); }
    true => { enable(); }
    null => { handleMissing(); }
}
```

#### 9.3.2 Wildcard

`_` matches any value and discards it. Commonly used as a catch-all arm.

```
match status {
    QuestStatus::Completed => { reward(player); }
    _ => { log("Not complete yet"); }
}
```

#### 9.3.3 Variable Binding

A bare identifier matches any value and binds it to a new variable within the arm body.

```
match damage {
    0 => { log("Miss!"); }
    amount => { log($"Took {amount} damage"); }
}
```

#### 9.3.4 Enum Destructuring

Qualified enum variants with parenthesized sub-patterns destructure variant payloads.

```
match result {
    Result::Ok(val) => { use(val); }
    Result::Err(err) => { log(err.message); }
}
```

#### 9.3.5 Nested Destructuring

Patterns nest arbitrarily — sub-patterns can themselves be enum destructuring, wildcards, literals, or bindings.

```
match result {
    Result::Ok(QuestStatus::InProgress(step)) => { log($"On step {step}"); }
    Result::Ok(QuestStatus::Completed) => { reward(player); }
    Result::Ok(_) => { log("Other OK status"); }
    Result::Err(err) => { log(err.message); }
}
```

#### 9.3.6 Or-Patterns

Multiple patterns separated by `|` share a single arm body.

```
match status {
    QuestStatus::Completed | QuestStatus::Failed(_) => { removeFromActive(); }
    _ => { log("Still in progress"); }
}
```

#### 9.3.7 Range Patterns

Inclusive ranges (`..=`) match any value within the range. Useful for numeric thresholds.

```
match health {
    0 => { die(); }
    1..=25 => { log("Critical!"); }
    26..=75 => { log("Wounded"); }
    _ => { log("Healthy"); }
}
```

---

## 10. Contracts

Contracts define a set of methods and/or operators that a type must implement. They serve the role of interfaces/traits
and are the foundation for bounded generics, operator overloading, and component polymorphism.

```
contract Interactable {
    fn onInteract(mut self, who: Entity);
}

contract Tradeable {
    fn getInventory(self) -> List<Item>;
    fn trade(mut self, item: Item, with: Entity);
}

// Implementation for a struct
impl Interactable for Merchant {
    fn onInteract(mut self, who: Entity) {
        // open trade dialog
    }
}

// Using contracts as bounds
fn interactWith(mut thing: Interactable) {
    thing.onInteract(player);
}
```

### 10.1 Builtin Contracts

These contracts are implicitly defined by the compiler and map to operator syntax or special behavior:

**Arithmetic operators:**

| Contract    | Operator     | Signature                   |
|-------------|--------------|-----------------------------|
| `Add<T, R>` | `+`          | `operator +(other: T) -> R` |
| `Sub<T, R>` | `-` (binary) | `operator -(other: T) -> R` |
| `Mul<T, R>` | `*`          | `operator *(other: T) -> R` |
| `Div<T, R>` | `/`          | `operator /(other: T) -> R` |
| `Mod<T, R>` | `%`          | `operator %(other: T) -> R` |
| `Neg<R>`    | `-` (unary)  | `operator -() -> R`         |
| `Not<R>`    | `!` (unary)  | `operator !() -> R`         |

**Comparison operators:**

| Contract | Operator | Signature                       |
|----------|----------|---------------------------------|
| `Eq<T>`  | `==`     | `operator ==(other: T) -> bool` |
| `Ord<T>` | `<`      | `operator <(other: T) -> bool`  |

> `!=` is auto-derived as `!(a == b)` from `Eq`. `>`, `<=`, `>=` are auto-derived from `Ord` and `Eq`. These cannot be
> overridden individually.

**Indexing operators:**

| Contract         | Operator           | Signature                        |
|------------------|--------------------|----------------------------------|
| `Index<K, V>`    | `x[k]` (read)      | `operator [](key: K) -> V`       |
| `IndexSet<K, V>` | `x[k] = v` (write) | `operator []=(key: K, value: V)` |

> A type may implement `Index` without `IndexSet` (read-only indexing). Attempting `x[k] = v` on a type that only
> implements `Index` is a compile error.

**Bitwise operators:**

| Contract       | Operator | Signature                    |
|----------------|----------|------------------------------|
| `BitAnd<T, R>` | `&`      | `operator &(other: T) -> R`  |
| `BitOr<T, R>`  | `\|`     | `operator \|(other: T) -> R` |

**Iteration:**

| Contract      | Behavior                        | Signature                          |
|---------------|---------------------------------|------------------------------------|
| `Iterable<T>` | Enables `for` loops (see 10.3)  | `fn iterator(self) -> Iterator<T>` |
| `Iterator<T>` | Produces elements one at a time | `fn next(mut self) -> T?`          |

**Conversion and special contracts:**

| Contract  | Behavior                   | Signature                    |
|-----------|----------------------------|------------------------------|
| `Into<T>` | Type conversion (see 10.2) | `fn into(self) -> T`         |
| `Error`   | Result `E` bound           | `fn message(self) -> string` |

> **Note:** When a user writes `operator +` in an `impl` block, the compiler automatically registers it as an
> implementation of the `Add` contract. Users never need to write `impl Add<...> for ...` directly.

**Compound assignment:** Operators `+=`, `-=`, `*=`, `/=`, `%=` are syntactic sugar. `a += b` desugars to `a = a + b`
and dispatches through the corresponding arithmetic contract. They are not independently overloadable.

### 10.2 Into\<T\> — Type Conversion

The `Into<T>` contract is the universal conversion mechanism. A type may implement `Into<T>` for multiple target types.

```
struct HealthInfo {
    current: int,
    max: int,
}

impl Into<string> for HealthInfo {
    fn into(self) -> string {
        $"{self.current}/{self.max}"
    }
}

impl Into<float> for HealthInfo {
    fn into(self) -> float {
        self.current / self.max
    }
}
```

**Calling convention:** Conversions are always invoked with an explicit type parameter on the call site:

```
let label = hp.into<string>();    // "75/100"
let ratio = hp.into<float>();     // 0.75
```

The `<T>` on the call disambiguates which `Into<T>` implementation to dispatch. There is no implicit conversion at
assignment or argument boundaries — the caller must be explicit.

**Exception — formattable strings and dialogue lines:** When an expression appears in an interpolation slot (`{expr}`
inside `$"..."` or dialogue text), the compiler implicitly calls `.into<string>()`. This is the only context where
`Into<T>` is invoked without an explicit call.

```
let hp = new HealthInfo { current: 75, max: 100 };
let msg = $"HP: {hp}";
// Equivalent to: $"HP: {hp.into<string>()}"
```

> **Note:** All primitive types (`int`, `float`, `bool`, `string`) have built-in `Into<string>` implementations provided
> by the compiler.

### 10.3 Iterable\<T\> — For Loop Support

The `Iterable<T>` and `Iterator<T>` contracts enable any type to be used with `for` loops.

`Iterable<T>` is implemented on the collection. It returns an `Iterator<T>`, which produces elements one at a time via
`next()`. When `next()` returns `null`, iteration ends.

```
// A for loop:
for item in collection {
    process(item);
}

// Desugars to:
{
    let mut _iter = collection.iterator();
    let mut _next = _iter.next();
    while _next != null {
        let item = _next!;
        process(item);
        _next = _iter.next();
    }
}
```

The following types have compiler-provided `Iterable<T>` implementations:

| Type           | Element Type | Behavior                                            |
|----------------|--------------|-----------------------------------------------------|
| `T[]`          | `T`          | Iterates elements in order                          |
| `Range<int>`   | `int`        | Iterates from start to end (exclusive or inclusive) |
| `Range<float>` | `float`      | Iterates in increments of 1.0                       |

User-defined types can implement `Iterable<T>` to participate in `for` loops:

```
impl Iterable<Entity> for Party {
    fn iterator(self) -> Iterator<Entity> {
        self.members.iterator()
    }
}

// Now usable in for loops:
for member in party {
    if let Option::Some(hp) = member[Health] {
        hp.current = min(hp.current + 10, hp.max);
    }
}
```

---

## 11. Generics

Type parameters can be unbounded or bounded by one or more contracts.

```
// Unbounded generic
fn first<T>(items: List<T>) -> T {
    items[0]
}

// Single bound
fn sum<T: Add<T, T>>(a: T, b: T) -> T {
    a + b
}

// Multiple bounds
fn process<T: Consumable + Tradeable>(item: T) {
    // item has methods from both contracts
}

// Generic contract
contract Consumable<T> {
    fn consume(mut self, who: Entity) -> T;
    fn getCharges(self) -> int;
}

impl Consumable<HealEffect> for HealthPotion {
    fn consume(mut self, who: Entity) -> HealEffect {
        HealEffect(self.healAmount)
    }
    fn getCharges(self) -> int {
        self.charges
    }
}
```

### 11.1 Compiler and Runtime Notes

The compiler performs generic validation at declaration sites (ensuring bounded type parameters are used correctly) and
at call sites (verifying concrete types satisfy bounds). Value types passed through generic parameters are **boxed** —
wrapped in a heap-allocated container with a type tag — to allow uniform representation.

At runtime, generic dispatch resolves through the **contract dispatch table**: a mapping from
`(concrete_type_tag, contract_id, method_slot)` to a method entry point. The spec does not mandate a specific dispatch
strategy — runtimes may use hash tables, vtable arrays, inline caches, or monomorphization as they see fit.

### 11.2 Generic Call Syntax

At call sites, type arguments are provided with `<T>` directly after the function or type name.

```
let item = first<Item>(inventory);
let result = parse<int>("42");

// Type arguments can be omitted when the compiler can infer them from the arguments:
let item = first(inventory);    // T inferred as Item from List<Item>
```

> **Parser disambiguation:** The parser distinguishes `f<T>(args)` from `a < b` by syntactic lookahead —
> it examines tokens after `<` to determine whether they form a type argument list closed by `>(`.

---

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

The parameter list is optional for `dlg` declarations. Both `dlg name { ... }` and `dlg name() { ... }` are valid when
there are no parameters. This is unique to `dlg` — functions (`fn`) always require parentheses.

```
dlg worldIntro {              // no parens — valid
    @Narrator The world awaits.
}

dlg worldIntro() {            // empty parens — also valid
    @Narrator The world awaits.
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

Transitions have two forms:

- `-> name` — No-argument transition. The target dialogue must have no required parameters.
- `-> name(args)` — Transition with arguments passed to the target dialogue.

```
dlg questIntro {
    @Narrator A great evil threatens the land.
    $ choice {
        "Tell me more" {
            -> questDetails               // no-arg transition
        }
        "Not interested" {
            @Narrator Very well. Perhaps another time.
            -> townSquare                 // no-arg transition
        }
    }
}

dlg shopEntry(player: Entity) {
    @Narrator You enter the shop.
    -> shopDialog(player)                 // transition with argument
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

### 13.9 Dialogue Suspension

Dialogue operations are **transition points** — the runtime suspends execution and yields control to the host engine.
The core dialogue functions live in the `Runtime` namespace and are provided by the runtime, not the script:

- `Runtime.say(speaker, text)` — Display a line of dialogue. Suspends until the host signals the player has advanced.
- `Runtime.say_localized(speaker, key, fallback)` — Localized variant. Same suspension semantics.
- `Runtime.choice(options)` — Present choices to the player. Suspends until the host signals which option was selected.

These functions are not callable directly from user code under the `Runtime` prefix — the compiler lowers dialogue
syntax
(`@Speaker text`, `$ choice { ... }`) into calls to these functions automatically. See §28.5 for the full lowering.

The host is responsible for presenting the dialogue UI, advancing text, and returning choice selections. The runtime
does not prescribe how the host implements these — only that the runtime suspends until the host responds. This follows
the suspend-and-confirm model (see IL spec §1.14.2).

### 13.10 Dialogue Line Semantics

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

## 14. Entities

Entities are game objects declared with the `entity` keyword. They combine properties, components (`use`), lifecycle
hooks (`on`), and methods. Entities lower to structs with component fields, auto-generated contract implementations, and
engine registrations.

### 14.1 Entity Declaration

```
entity Guard {
    // Properties (with defaults)
    name: string = "Guard",
    health: int = 80,
    maxHealth: int = 80,
    patrolRoute: List<vec2> = List::new(),

    // Components (extern, data-only — provided by host engine)
    use Speaker {
        displayName: "Guard",
    },
    use Sprite {
        texture: "res://sprites/guard.png",
    },
    use Collider {
        shape: "rect",
        width: 32,
        height: 48,
    },

    // Methods
    fn greet(self) -> string {
        $"Halt! I am {self.name}"
    }

    fn damage(mut self, amount: int) {
        self.health -= amount;
        if self.health <= 0 {
            Entity.destroy(self);
        }
    }

    fn heal(mut self, amount: int) {
        self.health = min(self.health + amount, self.maxHealth);
    }

    // Lifecycle hooks
    on create {
        log($"Guard spawned: {self.name}");
    }

    on interact(who: Entity) {
        -> guardDialog(self, who)
    }

    on destroy {
        dropLoot(self);
    }
}
```

### 14.2 Creating Entities

Entities are constructed with the `new` keyword and brace syntax. The compiler knows the type is an entity and generates
the appropriate IL (entity registration, component attachment, lifecycle hooks). Properties can be overridden.

```
let guard = new Guard {
    name: "Steve",
    patrolRoute: [new vec2 { x: 0, y: 0 }, new vec2 { x: 10, y: 0 }],
};

// Construct with all defaults
let defaultGuard = new Guard {};
```

### 14.3 Component Access

Components are accessed via `[]` indexing by type. Components are extern and data-only — script code reads and writes
their fields directly. For components declared in the entity definition, access is guaranteed non-null. For arbitrary
Entity references, component access returns `Option`.

```
// On a known entity type — guaranteed, no optional
guard[Sprite].visible = false;
guard[Collider].width = 48;

// On a generic Entity reference — returns Option
fn checkHealth(target: Entity) {
    if let Option::Some(hp) = target[Health] {
        if hp.current <= 0 {
            log("Target is dead");
        }
    }
}

// Unwrap if confident
let hp = target[Health]!.current;
```

> **Note:** If two components have a field with the same name, accessing it directly on the entity is a compile error.
> Use explicit component access: `self[Health].current` vs `self[Mana].current`.

### 14.4 Singleton Entities

Entities marked with the `[Singleton]` attribute are guaranteed to have at most one instance. They are accessed via
`Entity.getOrCreate<T>()`, which returns the existing instance or creates one. This is the mechanism used for
globally-referenced speakers in dialogue.

```
[Singleton]
entity Narrator {
    use Speaker {
        displayName: "The Narrator",
        color: "#CCCCCC",
    },
}

[Singleton]
entity OldTim {
    use Speaker {
        displayName: "Old Tim",
        color: "#AA8833",
        portrait: "res://portraits/tim.png",
    },
    use Sprite {
        texture: "res://sprites/tim.png",
    },
    gold: int = 500,

    on interact(who: Entity) {
        -> shopDialog(who)
    }
}

// Explicit access in code
let tim = Entity.getOrCreate<OldTim>();
tim.gold -= 10;

// In dialogue, @OldTim auto-resolves via Entity.getOrCreate<OldTim>()
dlg shopDialog(customer: Entity) {
    @Narrator You enter the shop.
    @OldTim Welcome, traveler!
    $ let tim = Entity.getOrCreate<OldTim>();
    $ tim.gold -= 10;
    @OldTim Here, a discount for you.
}
```

### 14.5 Entity References & EntityList

Entities reference each other by handle. The `EntityList<T>` type provides a typed collection for managing groups of
entities.

```
entity Party {
    leader: Player,
    members: EntityList<Entity> = EntityList::new(),

    fn addMember(mut self, e: Entity) {
        self.members.add(e);
    }

    fn healAll(self, amount: int) {
        for member in self.members {
            if let Option::Some(hp) = member[Health] {
                hp.current = min(hp.current + amount, hp.max);
            }
        }
    }
}
```

### 14.5.1 Entity Handles

Entity references are runtime-managed **handles** — opaque identifiers that the runtime resolves against its internal
entity registry. Unlike structs (which are direct GC references to heap objects), entity handles add an indirection
layer
because entities can be explicitly destroyed while other code still holds references to them.

After `Entity.destroy(entity)` is called:

- Existing handles are **not** invalidated or nulled. They remain valid values that can be stored, passed, and compared.
- Accessing fields, components, or methods through a dead handle **crashes the task** — same severity as unwrapping
  None.
- Use `Entity.isAlive(entity)` to check whether a handle refers to a live entity without crashing.

The GC manages the handle objects themselves. An entity's memory is only collected after it is both destroyed (or never
explicitly destroyed) AND unreachable from all GC roots. A dead handle that is still referenced keeps the handle object
alive in the GC, but the underlying entity state is gone.

```
let guard = new Guard {};
let ref = guard;                // both guard and ref hold handles to the same entity
Entity.destroy(guard);          // entity destroyed — on_destroy fires, marked dead
Entity.isAlive(ref);            // false
// ref.name;                    // would crash — dead handle
```

### 14.5.2 Entity Static Methods

The `Entity` namespace provides static methods for entity lifecycle and queries:

| Method               | Signature                            | Behavior                                                                   |
|----------------------|--------------------------------------|----------------------------------------------------------------------------|
| `Entity.destroy`     | `fn destroy(entity: Entity)`         | Destroy an entity. Fires `on destroy`, marks dead, notifies host.          |
| `Entity.isAlive`     | `fn isAlive(entity: Entity) -> bool` | Check if a handle refers to a live entity. Does not crash on dead handles. |
| `Entity.getOrCreate` | `fn getOrCreate<T>() -> T`           | Get or create a singleton entity (see §14.4).                              |
| `Entity.findAll`     | `fn findAll<T>() -> EntityList<T>`   | Find all live entities of a type.                                          |

`Entity.destroy` and `Entity.isAlive` lower to dedicated IL instructions (`DESTROY_ENTITY`, `ENTITY_IS_ALIVE`).
`Entity.getOrCreate` and `Entity.findAll` lower to `GET_OR_CREATE` and `FIND_ALL` respectively.

### 14.6 Lifecycle Hooks

Entities support all the universal lifecycle hooks (shared with structs) plus entity-specific hooks. All hooks receive
an implicit `mut self` parameter.

#### 14.6.1 Universal Hooks

| Hook             | When                                            | Purpose                                 |
|------------------|-------------------------------------------------|-----------------------------------------|
| `on create`      | After all fields and components are initialized | Post-initialization logic               |
| `on finalize`    | GC is about to collect the entity               | Last-chance cleanup of native resources |
| `on serialize`   | Before the entity is serialized                 | Park native state                       |
| `on deserialize` | After the entity is deserialized                | Recreate native state                   |

#### 14.6.2 Entity-Specific Hooks

| Hook                       | When                               | Purpose                                           |
|----------------------------|------------------------------------|---------------------------------------------------|
| `on destroy`               | `Entity.destroy(entity)` is called | Deterministic cleanup, loot drops, deregistration |
| `on interact(who: Entity)` | Host fires interaction event       | Game-specific interaction logic                   |

`on destroy` is distinct from `on finalize`. Destruction is explicit and deterministic — the script calls
`Entity.destroy(entity)`. Finalization is implicit and non-deterministic — the GC collects the object when it becomes
unreachable. An entity that is destroyed will eventually be finalized by the GC, but finalization may also occur without
explicit destruction (e.g., if all references to the entity are dropped).

**Entity cleanup ordering:** When an entity is explicitly destroyed, the sequence is:

1. `on destroy` — deterministic cleanup (loot drops, deregistration, game logic).
2. The entity is marked destroyed. Accessing a destroyed entity handle crashes the task.
3. Eventually, when the GC determines the entity is unreachable: `on finalize` — native resource cleanup.
4. Runtime removes the entity from internal bookkeeping.

This ordering ensures `on finalize` always runs after `on destroy`, which is the expected pattern: `on destroy` handles
game logic, `on finalize` handles native resource cleanup (file handles, connections, etc.).

**Hook failure semantics:** If a lifecycle hook crashes, the crash unwinds and terminates the calling task's call stack.
The runtime must log the failure to the host via the runtime logging interface (see IL spec §1.14.7). An `on destroy`
crash still marks the entity as destroyed — the entity does not "survive" a failed destructor.

### 14.7 Entity Lowering

An entity declaration lowers to a TypeDef with fields, component slots, methods, and lifecycle hook registrations.

#### 14.7.1 TypeDef Generation

Each `entity` declaration produces a TypeDef in the IL metadata with kind `Entity`. The TypeDef contains:

- **Fields:** All entity properties (`name: string`, etc.) become regular fields on the TypeDef, with default values
  stored in the metadata.
- **Component slots:** Each `use Component { ... }` declaration registers a component type index on the entity type.
  Component instances are allocated and attached by the host engine during `SPAWN_ENTITY` — they are not stored as
  inline fields on the entity struct.
- **Component overrides:** Field overrides specified in `use Health { current: 80, max: 80 }` are stored in the TypeDef
  metadata and applied to the component instance during entity construction.

```
// entity Guard { name: string = "Guard", health: int = 80, use Sprite { ... }, ... }
// produces:
//   TypeDef(Guard, kind=Entity)
//     fields: [name: string, health: int, maxHealth: int, ...]
//     component_slots: [Speaker, Sprite, Collider]
//     component_overrides: [Speaker.displayName="Guard", Sprite.texture="res://...", ...]
```

#### 14.7.2 Method Lowering

Entity methods lower to regular functions with the entity handle as explicit `self`:

```
// fn greet(self) -> string { $"Halt! I am {self.name}" }
// lowers to:
//   MethodDef(Guard::greet, params=[self: Guard], returns=string)
```

#### 14.7.3 Lifecycle Hook Lowering

Lifecycle hooks lower to registered callback functions with implicit `mut self`:

| Hook                               | Lowered Signature                                | Registration                                |
|------------------------------------|--------------------------------------------------|---------------------------------------------|
| `on create { ... }`                | `fn __on_create(mut self: Guard)`                | Called after field init during construction |
| `on interact(who: Entity) { ... }` | `fn __on_interact(mut self: Guard, who: Entity)` | Called by host via "fire event"             |
| `on destroy { ... }`               | `fn __on_destroy(mut self: Guard)`               | Called by `DESTROY_ENTITY`                  |
| `on finalize { ... }`              | `fn __on_finalize(mut self: Guard)`              | Called by GC before collection              |
| `on serialize { ... }`             | `fn __on_serialize(mut self: Guard)`             | Called before serialization snapshot        |
| `on deserialize { ... }`           | `fn __on_deserialize(mut self: Guard)`           | Called after deserialization restore        |

The runtime stores these as method indices in the TypeDef metadata. `INIT_ENTITY` invokes `__on_create`.
`DESTROY_ENTITY` invokes `__on_destroy`. The host fires `on_interact` through the runtime-host interface.

#### 14.7.4 Component Access Lowering

Component access via `[]` lowers to IL instructions based on context:

- `guard[Health]` on a known entity type (component declared in entity) → `GET_COMPONENT r_dst, r_guard, Health_type`.
  The compiler knows the component exists, so the result is `Health` (not `Option<Health>`).
- `target[Health]` on a generic `Entity` reference → `GET_COMPONENT r_dst, r_target, Health_type`. Returns
  `Option<Health>` because the entity may not have that component.

#### 14.7.5 Construction Sequence

`new Guard { name: "Steve" }` compiles to the following IL:

```
SPAWN_ENTITY  r0, Guard_type      // 1. Allocate entity, notify host to create components
                                   //    with defaults and overrides
LOAD_STRING   r1, "Steve"_idx     // 2. Load override value
SET_FIELD     r0, name_field, r1  // 3. Override entity field
INIT_ENTITY   r0                  // 4. Fire on_create (calls Guard::__on_create)
```

The full sequence:

1. **SPAWN_ENTITY** — allocates the entity object, notifies the host to create all declared component instances with
   their TypeDef defaults and component overrides, and registers the entity with the entity runtime. Does NOT fire
   `on_create`.
2. **SET_FIELD** (zero or more) — applies field overrides from the construction expression.
3. **INIT_ENTITY** — fires `on_create`. At this point, all fields and components are fully initialized.

---

## 15. Components

Components are data schemas for composable behaviors that can be attached to entities via `use`. Components are always
engine-provided (`extern`) and contain only field declarations — no methods. The host engine owns component storage and
behavior; the script language defines the schema for compile-time type checking and field access.

### 15.1 Component Declarations

```
extern component Sprite {
    texture: string,
    scale: float = 1.0,
    visible: bool = true,
}

extern component Collider {
    shape: string,
    width: float,
    height: float,
}

// The Speaker component is used for dialogue attribution
extern component Speaker {
    displayName: string,
    color: string = "#FFFFFF",
    portrait: string = "",
    voice: string = "",
}

extern component Health {
    current: int,
    max: int,
}
```

### 15.2 Component Access

Script code reads and writes component fields directly. Components have no script-defined methods — any logic involving
component data is written as entity methods or free functions.

```
// Direct field access
guard[Health].current -= 10;
if guard[Health].current <= 0 {
    Entity.destroy(guard);
}

// Reading component fields
let isVisible = guard[Sprite].visible;
guard[Sprite].texture = "res://sprites/guard_alert.png";
```

### 15.3 Runtime Behavior

Component field reads and writes on extern components are proxied through the host API. When script code writes
`guard[Sprite].visible = false`, the runtime sends the field change to the host engine, which updates the native
representation. The runtime suspends execution until the host confirms the change has been processed, ensuring
consistency with the game engine's logic loop.

> **Note:** Components are not GC-managed script objects. They are host-owned data accessed through the entity handle.
> The `self.entity` back-reference is a compiler-emitted hidden field (using an internal name like `@entity` that is
> unreachable from Writ source code). The compiler sets this field during `SPAWN_ENTITY` and uses it when lowering
> component access expressions. It is not a user-facing language feature.

---

## 16. Attributes

Attributes provide metadata on declarations using `[]` syntax. They are placed on the line before the declaration they
modify. The parser collects pending attributes and attaches them when it encounters the next declaration keyword.

### 16.1 Syntax

Attributes accept positional arguments, named arguments, or both. Positional arguments must appear before named
arguments.

```
// No arguments
[Singleton]
entity Narrator { ... }

// Positional argument
[Deprecated("Use NewMerchant instead")]
entity OldMerchant { ... }

// Named arguments
[Import("physics", symbol = "phys_raycast_2d")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Multiple attributes (separate lines)
[Singleton]
[Deprecated("Use NewMerchant instead")]
entity OldMerchant { ... }

// Multiple attributes (comma-separated)
[Singleton, Deprecated("Use NewMerchant")]
entity OldMerchant { ... }
```

### 16.2 Builtin Attributes

| Attribute             | Applies To         | Parameters                               | Effect                                                                                                       |
|-----------------------|--------------------|------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| `[Singleton]`         | entity             | *(none)*                                 | Enforces at most one instance. Enables `Entity.getOrCreate<T>()` and auto-resolution in `@speaker` dialogue. |
| `[Deprecated(msg)]`   | any declaration    | `msg`: string (positional)               | Compiler warning when referenced. Message shown in language server.                                          |
| `[Locale(tag)]`       | dlg                | `tag`: string (positional)               | Marks this `dlg` as a locale-specific structural override. See [Section 25](#25-localization).               |
| `[Import(lib, ...)]`  | extern declaration | See [Section 24.2](#242-library-imports) | Marks an extern as loaded from a native library rather than provided by the runtime.                         |
| `[Conditional(name)]` | fn                 | `name`: string (positional)              | Marks a function as a conditional override. See [Section 16.4](#164-conditional-compilation).                |

### 16.3 Parser Disambiguation

The `[` token at statement level could be either an attribute or an array expression. The parser resolves this by
checking whether the token after the closing `]` is a declaration keyword (`entity`, `fn`, `struct`, etc.). If yes, it
is an attribute. Otherwise, it is an expression. This requires only one token of lookahead past the `]`.

### 16.4 Conditional Compilation

The `[Conditional("name")]` attribute marks a function as a **conditional override**. The condition name is a string
that is either active or inactive at compile time (defined in `writ.toml` or via compiler flags).

**Rules:**

1. Every conditional function **must** have a non-conditional counterpart with the same name and signature. A
   conditional function without a fallback is a compile error.
2. When the named condition is active, the conditional version replaces the fallback at compile time. When inactive, the
   fallback stands and the conditional version is excluded entirely.
3. `[Conditional]` applies only to functions (`fn`). It cannot be used on structs, entities, components, or other
   declarations.
4. Multiple conditional overrides for the same function are allowed with different condition names, but at most one
   condition may be active for a given function at compile time. Overlapping active conditions on the same function
   signature is a compile error.

```
// Non-conditional fallback (always required)
fn rumbleController(intensity: float) {
    // generic fallback — could be a no-op
}

// PlayStation-specific override
[Conditional("playstation")]
fn rumbleController(intensity: float) {
    // DualSense haptics via native import
}

// Xbox-specific override
[Conditional("xbox")]
fn rumbleController(intensity: float) {
    // Xbox trigger rumble
}
```

```
// Debug logging — no-op fallback in release
fn writeDebugLine(msg: string) { }

[Conditional("debug")]
fn writeDebugLine(msg: string) {
    runtime.log(msg);
}
```

This model mirrors dialogue localization: the non-conditional function is the "default locale" and conditional overrides
are locale-specific translations. Code that calls `writeDebugLine(...)` always compiles — the compiler selects the
appropriate implementation based on active conditions.

Conditions are defined in `writ.toml` (see [Section 2.5](#25-conditions)) or passed as compiler flags.

---

## 17. Operators & Overloading

### 17.1 Operator Precedence (highest to lowest)

| Prec | Operators                    | Assoc | Notes                                        |
|------|------------------------------|-------|----------------------------------------------|
| 1    | `.` `[]` `!` `?`             | Left  | Member access, index, unwrap, null-propagate |
| 2    | `- (unary)` `! (unary)`      | Right | Negation, logical NOT                        |
| 3    | `*` `/` `%`                  | Left  | Multiplication, division, modulo             |
| 4    | `+` `-`                      | Left  | Addition, subtraction                        |
| 5    | `<<` `>>`                    | Left  | Bit shifts                                   |
| 6    | `&`                          | Left  | Bitwise AND                                  |
| 7    | `\|`                         | Left  | Bitwise OR                                   |
| 8    | `<` `>` `<=` `>=`            | Left  | Comparison                                   |
| 9    | `==` `!=`                    | Left  | Equality                                     |
| 10   | `&&`                         | Left  | Logical AND (short-circuit)                  |
| 11   | `\|\|`                       | Left  | Logical OR (short-circuit)                   |
| 12   | `..` `..=`                   | Left  | Range (exclusive, inclusive)                 |
| 13   | `=` `+=` `-=` `*=` `/=` `%=` | Right | Assignment                                   |

### 17.2 Overloading Syntax

Operators are overloaded inside `impl` blocks using the `operator` keyword. The compiler automatically maps these to the
corresponding builtin contract.

```
impl vec2 {
    // Binary operators
    operator +(other: vec2) -> vec2 {
        vec2 { x: self.x + other.x, y: self.y + other.y }
    }

    operator *(scalar: float) -> vec2 {
        vec2 { x: self.x * scalar, y: self.y * scalar }
    }

    operator %(other: vec2) -> vec2 {
        vec2 { x: self.x % other.x, y: self.y % other.y }
    }

    // Unary operators
    operator -() -> vec2 {
        vec2 { x: -self.x, y: -self.y }
    }

    // Comparison
    operator ==(other: vec2) -> bool {
        self.x == other.x && self.y == other.y
    }

    // Index read
    operator [](index: int) -> float {
        match index {
            0 => { self.x }
            1 => { self.y }
        }
    }

    // Index write
    operator []=(index: int, value: float) {
        match index {
            0 => { self.x = value; }
            1 => { self.y = value; }
        }
    }
}

// Implicitly registers: impl Add<vec2, vec2> for vec2,
//   impl Neg<vec2> for vec2, impl Index<int, float> for vec2, etc.
```

### 17.3 Compound Assignment

Compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`) are syntactic sugar. They are not independently
overloadable.

```
a += b;     // desugars to: a = a + b;   (uses Add)
a -= b;     // desugars to: a = a - b;   (uses Sub)
a *= b;     // desugars to: a = a * b;   (uses Mul)
a /= b;     // desugars to: a = a / b;   (uses Div)
a %= b;     // desugars to: a = a % b;   (uses Mod)
```

### 17.4 Derived Operators

Some operators are auto-derived from a base implementation and cannot be overridden individually:

| Derived  | Base         | Rule                |
|----------|--------------|---------------------|
| `a != b` | `Eq`         | `!(a == b)`         |
| `a > b`  | `Ord`        | `b < a`             |
| `a <= b` | `Eq` + `Ord` | `a < b \|\| a == b` |
| `a >= b` | `Eq` + `Ord` | `!(a < b)`          |

---

## 18. Error Handling

Writ separates error handling into distinct, non-overlapping mechanisms:

| Operator | Works On           | Behavior                          | Use When                   |
|----------|--------------------|-----------------------------------|----------------------------|
| `?`      | `Option<T>` / `T?` | Propagates `None`, unwraps `Some` | Chaining nullable access   |
| `!`      | Option and Result  | Unwraps or crashes the task       | Confident the value exists |
| `try`    | `Result<T, E>`     | Propagates `Err`, unwraps `Ok`    | Bubbling errors up in `fn` |
| `match`  | Both               | Explicit pattern matching         | Need to handle both paths  |

### 18.1 The ? Operator (Null Propagation)

Works exclusively on `Option<T>`. The containing function must return an `Option` type. Does NOT work on `Result`.

```
fn getHealerName(p: Party) -> string? {
    p.healer?.name   // if healer is None, returns None
}
```

### 18.2 The ! Operator (Unwrap or Crash)

Works on both Option and Result. Extracts the value or terminates the current task with a runtime error. In `dlg`
blocks, the runtime catches the crash and handles it (logging, fallback dialogue, etc.).

```
let healer = party.healer!;      // crash if None
let quest = loadQuest(id)!;      // crash if Err
```

### 18.3 The try Keyword (Error Propagation)

Works exclusively on `Result<T, E>`. The containing function must return a compatible Result. Does NOT work on Option.

```
fn loadBothQuests() -> Result<QuestPair, Error> {
    let a = try loadQuest("main_01");   // propagates Err
    let b = try loadQuest("side_03");   // propagates Err
    Result::Ok(QuestPair(a, b))
}
```

### 18.4 The Error Contract

```
contract Error {
    fn message(self) -> string;
}

struct QuestError {
    code: int,
    detail: string,
}

impl Error for QuestError {
    fn message(self) -> string {
        $"Quest error {self.code}: {self.detail}"
    }
}
```

### 18.5 Separation Summary

The `?` and `try` operators occupy entirely separate domains. There is no implicit conversion between Option and Result.
This eliminates a class of subtle bugs and keeps the mental model simple: `?` is for null, `try` is for errors, `!` is
for "I'm sure."

---

## 19. Nullability & Optionals

`T?` is syntactic sugar for `Option<T>`. All types are non-nullable by default. The `null` keyword is sugar for
`Option::None`.

```
struct Party {
    leader: Entity,         // never null
    healer: Entity?,        // may be null (= Option<Entity>)
}

// Safe access with ?
let name = party.healer?.name;       // string?

// Assert non-null with !
let name = party.healer!.name;       // string (crash if null)

// Pattern matching
match party.healer {
    Option::Some(h) => { h.heal(player); }
    Option::None => { log("No healer!"); }
}

// if let
if let Option::Some(h) = party.healer {
    h.heal(player);
}

// Null assignment
let mut healer: Entity? = null;      // null = Option::None
```

---

## 20. Concurrency

All function calls implicitly yield if needed (coroutine-based). Script authors do not think about async/await for
normal sequential code. Explicit concurrency primitives are provided for background tasks.

### 20.1 Execution Model

Every function is implicitly a coroutine. When the runtime encounters a blocking operation (`wait()`, `say()`, player
input), it yields control to the game engine. The engine resumes execution when appropriate. This is invisible to the
script author.

### 20.2 Concurrency Primitives

| Primitive        | Syntax                | Behavior                                                                                        |
|------------------|-----------------------|-------------------------------------------------------------------------------------------------|
| `spawn`          | `spawn expr`          | Starts a background task, returns a handle. Scoped to parent — auto-cancelled when parent ends. |
| `spawn detached` | `spawn detached expr` | Independent background task. Outlives parent scope.                                             |
| `join`           | `join handle`         | Wait for a spawned task to complete.                                                            |
| `cancel`         | `cancel handle`       | Hard-terminate a task. Runs `defer` blocks.                                                     |
| `defer`          | `defer { ... }`       | Cleanup code that runs on normal return or cancellation.                                        |

```
dlg boulderScene {
    @Narrator The ground shakes...
    $ let task = spawn moveBoulder(vec2 { x: 10.0, y: 5.0 });
    @Narrator Quick, get out of the way!
    $ choice {
        "Run!" {
            $ cancel task;
            @Narrator You dodge just in time.
        }
        "Stand firm" {
            $ join task;
            @Narrator The boulder settles into place.
        }
    }
}

fn moveBoulder(target: vec2) {
    defer { boulder.animation = "idle"; }
    boulder.animation = "rolling";
    lerp(boulder.position, target, 3.0);
}
```

### 20.3 Task Lifetime Rules

Scoped tasks (`spawn`) are automatically cancelled when their parent scope exits (normal return, `->` transition, or
cancellation). Detached tasks (`spawn detached`) run independently and must be explicitly cancelled or run to
completion.

---

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

## 22. Globals & Atomic Access

### 22.1 Global Variables

Global mutable state is declared with `global mut`. Global immutable values use `const`. Globals are visible throughout
their namespace.

```
// Immutable constant (compile-time known)
const MAX_REPUTATION: int = 100;

// Mutable global (requires explicit global mut)
global mut reputation: int = 0;
global mut questLog: Map<string, QuestStatus> = Map::new();
global mut partyMembers: EntityList<Entity> = EntityList::new();
```

### 22.2 Concurrency Safety

All reads and writes to `global mut` variables are implicitly serialized by the runtime. Individual read or write
operations are atomic. No manual locking is required for single-operation access.

```
// These are safe — each is a single atomic operation
reputation += 10;
let currentRep = reputation;
```

### 22.3 Atomic Blocks

For multi-step operations that must execute without interleaving from other tasks, use `atomic { }`. The runtime
guarantees no other task reads or writes the involved globals during an atomic block.

```
atomic {
    let old = reputation;
    reputation = old + bonus;
    if reputation > MAX_REPUTATION {
        reputation = MAX_REPUTATION;
    }
}

// Simple operations don't need atomic
reputation += 10;  // already atomic (single operation)
```

> **Note:** `atomic` blocks should be kept small. Long-running operations inside `atomic` will block all other tasks
> that access the same globals. The runtime may warn on `atomic` blocks that contain yield points (`wait`, `say`, etc.).

---

## 23. Modules & Namespaces

Every Writ source file belongs to a namespace. Namespaces organize declarations into logical groups and prevent name
collisions. Multiple files may contribute to the same namespace. Access across namespaces uses the `::` operator.

### 23.1 Declarative Namespace

The declarative form assigns the entire file to a single namespace:

```
// file: survival/potions.writ
namespace survival;

pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

pub fn heal(target: Entity, amount: int) {
    target[Health].current += amount;
}
```

**Rules:**

1. At most one declarative `namespace` statement per file.
2. Must appear before any declarations other than `using` statements.
3. All declarations in the file belong to the declared namespace.
4. The namespace name may be a qualified path (e.g., `namespace survival::items;`) to place the file in a nested
   namespace.
5. Does not support defining sub-namespaces within the file — for that, use block form (Section 23.2).

### 23.2 Block Namespace

The block form wraps declarations in a `namespace name { }` block and supports nesting:

```
namespace survival {
    pub struct HealthPotion {
        charges: int,
        healAmount: int,
    }

    namespace items {
        pub struct Bread {
            freshness: float,
        }

        pub struct Water {
            purity: float,
        }
    }
}
```

`HealthPotion` is `survival::HealthPotion`. `Bread` is `survival::items::Bread`.

**Rules:**

1. Declarative and block forms are **mutually exclusive** within a file — a file uses one or the other (or neither).
2. Multiple top-level block namespaces may appear in the same file.
3. Block namespaces may nest to arbitrary depth.
4. A namespace may span multiple files. Two files both contributing `namespace survival { ... }` merge their
   declarations into the same namespace.

### 23.3 Root Namespace

If a file contains no `namespace` declaration (neither declarative nor block), its declarations are in the **root
namespace**. Root namespace declarations are accessible without any `::` prefix from all other namespaces:

```
// file: globals.writ
// (no namespace declaration)

pub const MAX_LEVEL: int = 50;

// file: game/main.writ
namespace game;

fn example() {
    let cap = MAX_LEVEL;   // accessible without qualification — pub + root namespace
}
```

> **Note:** The root namespace is intended for small projects or truly global declarations. Larger projects should
> namespace everything.

### 23.4 `using` Declarations

The `using` keyword brings names from another namespace into scope, eliminating the need for `::` qualification:

```
// file: game/combat.writ
namespace game;

using survival;

fn example() {
    let pot = new HealthPotion { charges: 3, healAmount: 50 };
    heal(pot, 25);
}
```

Without the `using`, these would require `survival::HealthPotion` and `survival::heal`. Only `pub` declarations from
the target namespace are brought into scope.

#### 23.4.1 Alias Form

The alias form binds a namespace to a shorter name:

```
using items = survival::items;

fn example() {
    let bread = new items::Bread { freshness: 1.0 };
}
```

The alias does **not** bring individual names into scope — it only shortens the namespace prefix. `Bread` alone would
not resolve; `items::Bread` is required.

#### 23.4.2 Placement Rules

- In **declarative-form files**: `using` may appear before or after the `namespace` declaration, but must appear before
  any other declarations.
- In **block-form files**: `using` may appear at file level (before any namespace blocks) or inside a namespace block (
  scoped to that block).
- In **files with no namespace**: `using` must appear before any declarations.

```
// Declarative — using before or after namespace
using combat;
namespace survival;
using quest_system;

// ... declarations ...
```

```
// Block — using inside a namespace block
namespace game {
    using survival;
    using combat;

    fn example() {
        let pot = new HealthPotion { charges: 3, healAmount: 50 };
    }
}
```

#### 23.4.3 Scope of `using`

A `using` declaration is scoped to its enclosing context:

- File-level `using` (in declarative or no-namespace files): visible throughout the entire file.
- `using` inside a namespace block: visible only within that block and its nested blocks.

`using` does **not** re-export. A file that does `using survival;` makes `survival`'s names available locally, but
consumers of that file's namespace must add their own `using` or use `::` qualification.

### 23.5 Same-Namespace Visibility

`pub` declarations within the same namespace are visible to each other without `::` qualification, regardless of which
file they are defined in:

```
// file: survival/potions.writ
namespace survival;

pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

// file: survival/crafting.writ
namespace survival;

fn brewPotion() -> HealthPotion {
    // HealthPotion is visible — same namespace, pub, no :: needed
    new HealthPotion { charges: 3, healAmount: 50 }
}
```

Non-`pub` top-level declarations are file-local and not visible from other files, even within the same namespace.

### 23.6 Visibility Modifiers

Writ has two visibility keywords: `pub` and `priv`. Declarations default to **private**.

| Modifier | Meaning                                                                                   |
|----------|-------------------------------------------------------------------------------------------|
| (none)   | **Private** — file-local for top-level declarations, type-private for members             |
| `priv`   | **Private** (explicit) — same as no modifier, for when the author wants to be intentional |
| `pub`    | **Public** — visible outside the file/type, accessible via `::` or `using`                |

#### 23.6.1 Top-Level Declarations

Top-level declarations (`fn`, `struct`, `enum`, `contract`, `entity`, `component`, `const`, `global`) default to
**private**, meaning they are visible only within the declaring file. `pub` makes them visible to all files and
namespaces.

```
namespace survival;

// Private (default) — only visible within this file
struct PotionRecipe {
    ingredients: string[],
    brewTime: float,
}

// Public — visible to any file via survival::HealthPotion or `using survival;`
pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

// Public — visible to any namespace
pub fn heal(target: Entity, amount: int) {
    target[Health].current += amount;
}

// Private — helper function, file-local
fn calculateHealAmount(level: int) -> int {
    level * 10
}

// Explicit priv — identical to no modifier, expresses intent
priv fn internalHelper() {
    // ...
}
```

From another file in the same namespace:

```
// file: survival/crafting.writ
namespace survival;

fn example() {
    let pot = new HealthPotion { charges: 3, healAmount: 50 };   // OK — HealthPotion is pub
    heal(player, 25);                                           // OK — heal is pub
    let r = new PotionRecipe { ingredients: [], brewTime: 5.0 };  // ERROR — PotionRecipe is private to its file
}
```

From another namespace:

```
namespace game;
using survival;

fn example() {
    heal(player, 25);                       // OK — heal is pub
    let pot = new HealthPotion {};           // OK — HealthPotion is pub
    let pot = new survival::HealthPotion {}; // OK — fully qualified also works
    calculateHealAmount(5);             // ERROR — private, not visible outside its file
}
```

**Exception — `dlg` declarations default to `pub`.** Dialogue blocks are intended to be called from other files and
namespaces (via transitions, entity hooks, or direct invocation). A `dlg` can be made private with an explicit `priv`:

```
namespace quest;

// Public by default — can be called from other files and namespaces
dlg mainQuest(player: Entity) {
    @Narrator Your adventure begins.
}

// Explicitly private — only used within this file as a helper
priv dlg internalBranch() {
    @Narrator This is an internal branch.
}
```

#### 23.6.2 Type Members

Members of structs, entities, and components (fields, properties, and methods) default to **type-private** — only the
type's own methods can access them. `pub` makes members visible wherever the type itself is visible.

```
pub struct Merchant {
    pub name: string,             // public — accessible wherever Merchant is visible
    gold: int,                    // private — only Merchant's own methods can access
    priv discount: float = 0.1,  // private (explicit) — same as no modifier
}

impl Merchant {
    pub fn greet(self) -> string {
        $"Welcome! I am {self.name}"
    }

    fn applyDiscount(self, price: int) -> int {
        // Can access private fields — we're inside the type
        price - (price * self.discount)
    }
}
```

```
namespace survival;

fn example(m: Merchant) {
    let n = m.name;               // OK — pub
    let g = m.gold;               // ERROR — private, only Merchant methods can access
    let d = m.discount;           // ERROR — private
}
```

#### 23.6.3 Entity and Component Members

Entities and components follow the same rules as structs:

```
pub entity Guard {
    pub name: string = "Guard",
    alertLevel: int = 0,

    use Speaker {
        displayName: "Guard",
    },
    use Health {
        current: 80,
        max: 80,
    },

    pub fn greet(self) -> string {
        $"Halt! I am {self.name}"
    }

    fn raiseAlert(mut self) {
        self.alertLevel += 1;
    }

    on interact(who: Entity) {
        self.raiseAlert();        // OK — on hooks are part of the type
        -> guardDialog(self, who)
    }
}
```

Lifecycle hooks (`on`) do not take visibility modifiers — they are always type-private (invoked by the runtime, not
called by user code).

Component `use` declarations do not take visibility modifiers — component attachment is visible wherever the entity is
visible. Component field visibility is governed by the component's own declarations.

#### 23.6.4 Contracts and Implementations

Contract method signatures do not take visibility modifiers. Contract methods define a public interface — any type
implementing the contract must expose those methods publicly:

```
contract Tradeable {
    fn getInventory(self) -> List<Item>;    // no modifier — always part of the public interface
    fn trade(mut self, item: Item, with: Entity);
}
```

Methods in `impl` blocks that fulfill a contract requirement are implicitly `pub` and cannot be made private:

```
impl Tradeable for Merchant {
    fn getInventory(self) -> List<Item> { ... }   // OK — implicitly pub
    fn trade(mut self, item: Item, with: Entity) { ... }

    priv fn getInventory(self) -> List<Item> { ... }   // ERROR — contract methods cannot be private
}
```

Additional non-contract methods in an `impl` block follow normal visibility rules:

```
impl Merchant {
    pub fn greet(self) -> string { ... }
    fn calculateMarkup(self) -> float { ... }    // private — only Merchant can call this
}
```

#### 23.6.5 Enum Variants

Enum variants do not take individual visibility modifiers. All variants share the visibility of the enum itself:

```
pub enum QuestStatus {
    NotStarted,                    // all variants are pub because the enum is pub
    InProgress(currentStep: int),
    Completed,
    Failed(reason: string),
}
```

#### 23.6.6 Visibility Summary

| Declaration context           | `pub`  | (none) / `priv` |
|-------------------------------|--------|-----------------|
| Top-level (`fn`, `struct`, …) | Public | File-local      |
| Top-level `dlg`               | Public | File-local*     |
| Struct field                  | Public | Type-private    |
| Struct method (in `impl`)     | Public | Type-private    |
| Entity property               | Public | Type-private    |
| Entity method                 | Public | Type-private    |
| Entity lifecycle hook (`on`)  | —      | Always internal |
| Component field               | Public | Type-private    |
| Contract method signature     | —      | Always public   |
| Contract impl method          | —      | Always public   |
| Enum variant                  | —      | Inherits enum   |

*`dlg` defaults to `pub`; an explicit `priv` makes it file-local. All other declarations default to private.

### 23.7 Name Conflicts

If two namespaces define a type or function with the same name, and both are brought into scope via `using`, any
**unqualified** reference to that name is a compile error:

```
namespace ns_a;
pub struct Item { name: string }

namespace ns_b;
pub struct Item { id: int }
```

```
// file: main.writ
namespace main;

using ns_a;
using ns_b;

fn example() {
    let x = Item();         // ERROR: ambiguous — Item exists in both ns_a and ns_b
    let y = ns_a::Item();   // OK — fully qualified
    let z = ns_b::Item();   // OK — fully qualified
}
```

The error occurs at the **usage site**, not at the `using` declaration. Having two `using` statements that *could*
conflict is legal as long as no ambiguous name is actually used without qualification.

> **Note:** Only `pub` declarations are visible outside their declaring file. A `using` only brings `pub` declarations
> into scope. Private declarations are never accessible from other files, even with `::` qualification.

### 23.8 Cross-Namespace Access

The `::` operator accesses `pub` names within a namespace:

```
let pot = new survival::HealthPotion { charges: 3, healAmount: 50 };
let bread = new survival::items::Bread { freshness: 1.0 };
survival::heal(player, 25);
```

Fully qualified names always work for `pub` declarations, regardless of `using` declarations. They also resolve
ambiguity when multiple `using` statements bring conflicting names into scope. Private declarations cannot be accessed
via `::` from outside their file.

### 23.9 Root Namespace Prefix (`::`)

A leading `::` with no left-hand side refers to the root namespace. This resolves ambiguity when a nested namespace
shadows an outer one:

```
namespace engine {
    namespace audio {
        pub struct Mixer { channels: int }
    }
}

namespace audio {
    pub struct Mixer { sampleRate: int }
}
```

```
namespace engine::audio;

fn example() {
    // "audio" here resolves to engine::audio (inner takes priority)
    let a = Mixer(channels: 8);

    // Leading :: forces resolution from the root
    let b = ::audio::Mixer(sampleRate: 44100);
}
```

**Resolution rule:** When an unqualified name could refer to either a sibling/child namespace or a root-level namespace,
the **innermost** (closest enclosing) match takes priority. Use `::name` to bypass this and start resolution from the
root.

The leading `::` works in all expression and type contexts:

```
let x = new ::survival::HealthPotion { charges: 3, healAmount: 50 };
let y: ::survival::HealthPotion = x;
::survival::heal(player, 25);
```

### 23.10 `::` Resolution

The `::` operator is used in three contexts:

1. **Root namespace access** — `::survival::HealthPotion` (leading `::`, resolve from root)
2. **Namespace access** — `survival::HealthPotion`, `survival::items::Bread`
3. **Enum variant access** — `QuestStatus::InProgress`, `Option::Some(value)`

The compiler resolves `::` by checking whether the left-hand side names a namespace or a type. If it names a namespace,
namespace lookup is performed. If it names an enum type, variant lookup is performed. A leading `::` (no left-hand side)
always starts from the root namespace. This is always unambiguous because namespaces and types occupy separate name
spaces — a namespace `Option` and an enum `Option` cannot coexist (this would be a name conflict).

### 23.11 File Path Convention

Namespace structure **should** mirror the directory structure. This is a recommended convention, not a compiler-enforced
rule:

| Namespace         | Recommended Path                                 |
|-------------------|--------------------------------------------------|
| `survival`        | `survival/*.writ` or `survival.writ`             |
| `survival::items` | `survival/items/*.writ` or `survival/items.writ` |
| `quest_system`    | `quest_system/*.writ`                            |

The compiler does not validate that file paths match namespace declarations. A file at `combat/spells.writ` may declare
`namespace ui;` without error. However, violating the convention makes the project harder to navigate and the language
server flags it as a warning.

> **Note:** All files in the project are gathered and indexed before compilation. The compiler discovers all `.writ`
> files in the project directory (as defined by `writ.toml`) and uses namespace declarations — not file paths — for
> symbol
> resolution.

---

## 24. External Declarations

External declarations describe types, functions, components, and other constructs not implemented in Writ. They have no
implementation body and exist for compile-time type checking and language server support. External declarations are
placed in regular `.writ` files. By convention, projects organize them in a `decl/` directory, but this is not required.

There are two kinds of external declarations:

1. **Runtime-provided** — bare `extern` with no `[Import]` attribute. The host runtime supplies the implementation at
   embedding time.
2. **Library-imported** — `extern` with an `[Import]` attribute. The runtime loads a native library and resolves the
   symbol at call time.

### 24.1 Runtime-Provided Externals

Bare `extern` declarations are provided by the host runtime. This is the common case for game scripting — the engine
exposes core functionality to scripts.

```
// Runtime-provided functions
extern fn lerp(from: vec2, to: vec2, duration: float) -> vec2;
extern fn wait(seconds: float);
extern fn playSound(name: string);
extern fn random(min: float, max: float) -> float;

// Runtime-provided structs
extern struct vec2 {
    x: float,
    y: float,
}

extern struct Entity {
    position: vec2,
    name: string,
    fn moveTo(target: vec2, speed: float);
    fn destroy();
}

// Runtime-provided components (data-only — no methods)
extern component Sprite {
    texture: string,
    scale: float = 1.0,
    visible: bool = true,
}

extern component Speaker {
    displayName: string,
    color: string = "#FFFFFF",
    portrait: string = "",
    voice: string = "",
}

extern component Health {
    current: int,
    max: int,
}

// Entity namespace utilities
extern fn Entity.getOrCreate<T>() -> T;
extern fn Entity.findAll<T>() -> EntityList<T>;
extern fn Entity.findNearest<T>(position: vec2) -> T?;
```

### 24.2 Library Imports

The `[Import]` attribute marks an extern declaration as loaded from a native library rather than provided directly by
the runtime.

```
[Import("physics")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;
```

#### 24.2.1 Import Attribute Parameters

The `[Import]` attribute accepts one positional argument (the logical library name) and optional named arguments for
symbol naming and architecture-specific overrides.

**Library name parameters:**

| Parameter      | Type   | Description                                                                 |
|----------------|--------|-----------------------------------------------------------------------------|
| *(positional)* | string | Logical library name. Resolved by the runtime or via `writ.toml`. Required. |
| `x86`          | string | Library name override for x86 architecture.                                 |
| `x64`          | string | Library name override for x64 architecture.                                 |
| `arm`          | string | Library name override for arm architecture.                                 |
| `arm64`        | string | Library name override for arm64 architecture.                               |
| `wasm32`       | string | Library name override for wasm32 architecture.                              |

**Symbol name parameters:**

| Parameter       | Type   | Description                                                                |
|-----------------|--------|----------------------------------------------------------------------------|
| `symbol`        | string | Symbol name in the library. Defaults to the Writ function name if omitted. |
| `symbol_x86`    | string | Symbol name override for x86 architecture.                                 |
| `symbol_x64`    | string | Symbol name override for x64 architecture.                                 |
| `symbol_arm`    | string | Symbol name override for arm architecture.                                 |
| `symbol_arm64`  | string | Symbol name override for arm64 architecture.                               |
| `symbol_wasm32` | string | Symbol name override for wasm32 architecture.                              |

These parameters form a closed set. The compiler rejects unrecognized named arguments in `[Import]`.

#### 24.2.2 Examples

```
// Minimal — logical name only, symbol defaults to function name
[Import("physics")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Custom symbol name (library exports a different name than the Writ function)
[Import("physics", symbol = "phys_raycast_2d")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Architecture-specific library names
[Import("physics", x64 = "physics64", arm64 = "physics_arm")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Architecture-specific symbol names (name mangling differences)
[Import("physics", symbol = "raycast", symbol_x64 = "_raycast@24")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Full override example
[Import("audio", x64 = "fmod64", arm64 = "fmod_arm", symbol = "FMOD_PlaySound")]
extern fn playMusic(path: string, volume: float);
```

### 24.3 Architecture Identifiers

The following architecture identifiers are recognized by the compiler:

| Identifier | Architecture                        |
|------------|-------------------------------------|
| `x86`      | 32-bit Intel / AMD                  |
| `x64`      | 64-bit Intel / AMD (x86_64 / AMD64) |
| `arm`      | 32-bit ARM                          |
| `arm64`    | 64-bit ARM (AArch64)                |
| `wasm32`   | 32-bit WebAssembly                  |

Unrecognized architecture identifiers in `[Import]` named parameters are a compile error.

> **Note:** Architecture identifiers refer to instruction set architecture only. Platform concerns (operating system,
> file extensions, library search paths) are the runtime's responsibility.

### 24.4 Library Resolution

When the runtime encounters a call to an `[Import]` extern, it resolves the library in the following order:

1. **Attribute architecture override** — if the current architecture has a named override (e.g., `x64 = "physics64"`),
   use that name.
2. **`writ.toml` libraries section** — if the project defines a `[libraries.<name>]` entry (
   see [Section 2](#2-project-configuration-writtoml)), use that mapping.
3. **Logical name** — use the positional argument as-is.

The runtime appends platform-specific file extensions (`.dll`, `.so`, `.dylib`) and applies its own search path
conventions. The Writ language does not specify file extensions or search paths — these are runtime concerns.

### 24.5 Symbol Resolution

Symbol resolution follows the same precedence:

1. **Attribute architecture override** — if the current architecture has a symbol override (e.g.,
   `symbol_x64 = "_raycast@24"`), use that name.
2. **Attribute symbol parameter** — if `symbol` is specified, use that name.
3. **Function name** — default to the Writ function name as declared.

### 24.6 Crash Semantics

Library loading and symbol resolution are **not recoverable operations**. If the runtime cannot load a library or
resolve a symbol:

1. The runtime MUST terminate the current task.
2. All `defer` blocks in the call chain unwind and execute, in reverse order (same as cancellation).
3. The crash propagates through the entire task chain — parent tasks that spawned the failing task are also terminated.

This is an unrecoverable error, not a `Result`. Script code cannot catch or recover from a failed library load. The
runtime MAY reject a library load for any reason, including security policy (e.g., unsigned libraries, disallowed paths,
sandboxing). The behavior is the same: crash with defer unwinding.

> **Rationale:** Library imports are an injection surface. The runtime is the gatekeeper — it decides which libraries
> are permitted. Making failures unrecoverable prevents scripts from silently falling back to alternate code paths when
> a
> library is blocked, which could mask security violations.

---

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

## 26. Standard Library Builtins

### 26.1 Compiler-Known Types

| Type           | Sugar        | Purpose                                 |
|----------------|--------------|-----------------------------------------|
| `Option<T>`    | `T?`, `null` | Nullable values                         |
| `Result<T, E>` | —            | Fallible operations (`E: Error`)        |
| `Range<T>`     | `..`, `..=`  | Interval type for iteration and slicing |

### 26.2 Compiler-Known Contracts

| Contract                          | Special Behavior                                                                                                                                                                              |
|-----------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Error`                           | Required bound for Result's `E` parameter. Requires `message() -> string`.                                                                                                                    |
| `Into<T>`                         | Type conversion. Called explicitly via `.into<T>()`. Implicitly called as `.into<string>()` by `{expr}` interpolation in formattable strings (`$"..."`) and dialogue lines. See Section 10.2. |
| `Add`, `Sub`, `Mul`, `Div`, `Mod` | Mapped from `operator +`, `-`, `*`, `/`, `%` syntax.                                                                                                                                          |
| `Neg`, `Not`                      | Mapped from unary `-` and `!` syntax.                                                                                                                                                         |
| `Eq`, `Ord`                       | Mapped from `operator ==`, `<`. Derived: `!=`, `>`, `<=`, `>=`.                                                                                                                               |
| `Index<K, V>`, `IndexSet<K, V>`   | Mapped from `operator []` (read) and `operator []=` (write) syntax.                                                                                                                           |
| `BitAnd`, `BitOr`                 | Mapped from `operator &`, `\|`.                                                                                                                                                               |
| `Iterable<T>`, `Iterator<T>`      | Enable `for` loop iteration. `T[]` and `Range<T>` have compiler-provided implementations. See Section 10.3.                                                                                   |

### 26.3 Standard Library Types

These types are provided by the standard library with no special compiler support:

| Type            | Description                                                    |
|-----------------|----------------------------------------------------------------|
| `List<T>`       | Ordered, growable collection                                   |
| `Map<K, V>`     | Key-value associative collection                               |
| `Set<T>`        | Unordered unique collection                                    |
| `EntityList<T>` | Typed entity reference collection with component query support |

---

## 27. Grammar Summary (EBNF)

A simplified EBNF sketch of the core grammar. Not exhaustive but captures key structural rules.

```ebnf
program        = { using_decl | namespace_decl | declaration } ;
namespace_decl = 'namespace' qualified_name ( ';'
               | '{' { using_decl | namespace_decl | declaration } '}' ) ;
using_decl     = 'using' ( IDENT '=' )? qualified_name ';' ;
qualified_name = IDENT { '::' IDENT } ;
rooted_name    = [ '::' ] qualified_name ;  /* leading :: = root namespace */

visibility     = 'pub' | 'priv' ;
declaration    = { attribute } [ visibility ] ( fn_decl | dlg_decl | struct_decl
               | enum_decl | contract_decl | impl_decl
               | entity_decl | extern_decl
               | const_decl | global_decl ) ;

attribute      = '[' attr_item { ',' attr_item } ']' ;
attr_item      = IDENT [ '(' [ attr_args ] ')' ] ;
attr_args      = attr_arg { ',' attr_arg } ;
attr_arg       = IDENT '=' expr       /* named argument */
               | expr ;               /* positional argument */

fn_decl        = 'fn' IDENT [ generic_params ] '(' [ params ] ')'
                 [ '->' type ] block ;
dlg_decl       = 'dlg' IDENT [ '(' [ params ] ')' ] dlg_block ;

struct_decl    = 'struct' IDENT [ generic_params ] '{'
                 { struct_member } '}' ;
struct_member  = [ visibility ] property | on_decl ;
enum_decl      = 'enum' IDENT [ generic_params ] '{'
                 { variant ',' } '}' ;
variant        = IDENT [ '(' { IDENT ':' type ',' } ')' ] ;

contract_decl  = 'contract' IDENT [ generic_params ] '{'
                 { fn_sig | op_sig } '}' ;
impl_decl      = 'impl' [ contract 'for' ] type '{'
                 { [ visibility ] ( fn_decl | op_decl ) } '}' ;

entity_decl    = 'entity' IDENT '{' { entity_member } '}' ;
entity_member  = [ visibility ] property | use_decl
               | [ visibility ] fn_decl | on_decl ;
property       = IDENT ':' type [ '=' expr ] ',' ;
use_decl       = 'use' IDENT [ '{' { IDENT ':' expr ',' } '}' ] ',' ;
on_decl        = 'on' IDENT [ '(' params ')' ] block ;

/* Components are always extern — see extern_decl */
component_decl = 'component' IDENT '{' { [ visibility ] property } '}' ;

extern_decl    = 'extern' ( fn_sig ';' | struct_decl
               | component_decl ) ;

const_decl     = 'const' IDENT ':' type '=' expr ';' ;
global_decl    = 'global' 'mut' IDENT ':' type '=' expr ';' ;

/* Construction expression */
new_expr       = 'new' rooted_name [ '<' type { ',' type } '>' ]
                 '{' { IDENT ':' expr ',' } '}' ;

/* Lambdas (anonymous functions) */
lambda         = 'fn' '(' [ lambda_params ] ')' [ '->' type ] block ;
lambda_params  = lambda_param { ',' lambda_param } ;
lambda_param   = IDENT [ ':' type ] ;

/* String literals */
string_literal = basic_string | formattable_string
               | raw_string | formattable_raw_string ;
basic_string   = '"' { char | escape } '"' ;
formattable_string = '$"' { char | escape | interpolation } '"' ;
raw_string     = QUOTES_N NEWLINE { raw_char } QUOTES_N ;
                 /* QUOTES_N = 3+ consecutive '"' chars; same count opens and closes */
formattable_raw_string = '$' QUOTES_N NEWLINE { raw_char | interpolation } QUOTES_N ;
interpolation  = '{' expr '}' ;

/* Range expressions */
range_expr     = [ expr ] ( '..' | '..=' ) [ expr ] ;
from_end_index = '^' expr ;    /* only valid inside [] */

/* Array literals */
array_literal  = '[' [ expr { ',' expr } ] ']' ;

/* Variables */
var_decl       = 'let' [ 'mut' ] IDENT [ ':' type ] '=' expr ';' ;

/* Generics */
generic_params = '<' IDENT [ ':' bound ]
                 { ',' IDENT [ ':' bound ] } '>' ;
bound          = IDENT [ '<' type { ',' type } '>' ]
                 { '+' IDENT [ '<' type { ',' type } '>' ] } ;

/* Dialogue blocks */
dlg_block      = '{' { dlg_line } '}' ;
dlg_line       = speaker_line | dlg_escape | transition | text_line ;
speaker_line   = '@' IDENT [ text_content [ '#' IDENT ] ] NEWLINE ;
text_line      = text_content [ '#' IDENT ] NEWLINE ;
dlg_escape     = '$' ( dlg_choice | dlg_if | dlg_match
               | block | statement ) ;
dlg_choice     = 'choice' '{' { STRING [ '#' IDENT ] dlg_block } '}' ;
dlg_if         = 'if' expr dlg_block [ 'else' ( dlg_if | dlg_block ) ] ;
dlg_match      = 'match' expr '{' { pattern '=>' dlg_block } '}' ;
transition     = '->' IDENT ;

/* Patterns (used in match arms and if-let) */
pattern        = literal_pat | wildcard_pat | enum_pat | or_pat
               | range_pat | binding_pat ;
literal_pat    = INT_LIT | STRING_LIT | 'true' | 'false' | 'null' ;
wildcard_pat   = '_' ;
binding_pat    = IDENT ;          /* matches anything, binds to name */
enum_pat       = rooted_name '(' [ pattern { ',' pattern } ] ')' ;
or_pat         = pattern '|' pattern { '|' pattern } ;
range_pat      = INT_LIT '..=' INT_LIT ;

/* Statements */
statement      = var_decl | expr_stmt | for_stmt | while_stmt
               | 'break' ';' | 'continue' ';' | 'return' [ expr ] ';' ;
expr_stmt      = expr ';' ;

/* Blocks and control flow */
block          = '{' { statement | block_expr } [ expr ] '}' ;
block_expr     = if_expr | match_expr ;   /* no trailing ; required */
if_expr        = 'if' expr block [ 'else' ( if_expr | block ) ] ;
match_expr     = 'match' expr '{' { pattern '=>' block } '}' ;
for_stmt       = 'for' IDENT 'in' expr block ;
while_stmt     = 'while' expr block ;

/* Expressions (simplified — see Section 17.1 for full precedence) */
expression     = literal | IDENT | unary_expr | binary_expr | call_expr
               | member_expr | index_expr | if_expr | match_expr
               | lambda | block | range_expr | array_literal
               | new_expr ;
call_expr      = expr [ '<' type { ',' type } '>' ] '(' [ args ] ')' ;
args           = arg { ',' arg } ;
arg            = [ IDENT ':' ] expr ;     /* positional or named */
member_expr    = expr '.' IDENT ;
index_expr     = expr '[' expr ']' ;
unary_expr     = ( '-' | '!' | 'try' ) expr ;
binary_expr    = expr BINARY_OP expr ;    /* see Section 17.1 for operators */

/* Types */
type           = IDENT [ '<' type { ',' type } '>' ] [ '[]' ] [ '?' ] ;
```

---

## 28. Lowering Reference

All higher-level constructs lower to simpler primitives before execution.

### 28.1 Dialogue Lowering

| Dialogue Construct       | Lowers To                                          |
|--------------------------|----------------------------------------------------|
| `@speaker Text.`         | `say(speaker, "Text.");`                           |
| `@speaker` (default set) | Subsequent lines use set speaker in `say()` calls  |
| `$ choice { ... }`       | `choice([ ... ]);`                                 |
| `$ if cond { ... }`      | `if cond { ... }` (branches contain `say()` calls) |
| `$ match expr { ... }`   | `match expr { ... }` (arms contain `say()` calls)  |
| `-> otherDialog`         | `return otherDialog();`                            |
| `$ statement;`           | `statement;`                                       |
| `{expr}` in text         | Concatenation with `.into<string>()`               |
| `#key` on text line      | Overrides auto-generated key in `say_localized()`  |
| `@Singleton` (auto)      | `Entity.getOrCreate<T>()` for speaker              |

### 28.2 Full Dialogue Lowering Example

Source:

```
dlg greetPlayer(name: string) {
    @Narrator Hey, {name}.
    @Narrator
    How are you?
    $ choice {
        "Good!" {
            $ reputation += 1;
            Glad to hear it.
        }
        "Not great" {
            @Player Things are rough.
            @Narrator Sorry to hear that.
        }
    }
    -> farewellDialog
}
```

Lowered output:

```
fn greetPlayer(name: string) {
    let _narrator = Entity.getOrCreate<Narrator>();
    let _player = Entity.getOrCreate<Player>();
    say(_narrator, "Hey, " + name.into<string>() + ".");
    say(_narrator, "How are you?");
    choice([
        Option("Good!", fn() {
            reputation += 1;
            say(_narrator, "Glad to hear it.");
        }),
        Option("Not great", fn() {
            say(_player, "Things are rough.");
            say(_narrator, "Sorry to hear that.");
        }),
    ]);
    return farewellDialog();
}
```

### 28.3 Entity Lowering

Entities lower to TypeDefs (kind=Entity) with fields, component slots, methods, and lifecycle hooks. Components are
extern and data-only — they are host-managed instances attached to the entity.

```
// entity Guard { name: string = "Guard", health: int = 80, use Sprite { ... }, ... }
// produces:
//   TypeDef(Guard, kind=Entity)
//     fields: [name: string, health: int, maxHealth: int, ...]
//     component_slots: [Speaker, Sprite, Collider]
//     component_overrides: [Speaker.displayName="Guard", Sprite.texture="res://...", ...]
//     lifecycle: [on_create, on_destroy, on_finalize, on_serialize, on_deserialize]

// Methods lower to:
//   fn Guard::greet(self: Guard) -> string { ... }
//   fn Guard::__on_create(mut self: Guard) { ... }     // from: on create { ... }
//   fn Guard::__on_interact(mut self: Guard, who: Entity) { ... }
//   fn Guard::__on_destroy(mut self: Guard) { ... }
//   fn Guard::__on_finalize(mut self: Guard) { ... }
//   fn Guard::__on_serialize(mut self: Guard) { ... }
//   fn Guard::__on_deserialize(mut self: Guard) { ... }
```

See [Section 14.7](15_14_entities.md#147-entity-lowering) for the full lowering specification.

### 28.4 Localized Dialogue Lowering

When localization is active, `say()` calls include key metadata for runtime lookup:

```
// say(_narrator, "Hey, " + name.into<string>() + ".");
// becomes:
say_localized(
    _narrator,
    "a3f7c012",                        // pre-computed FNV-1a key
    "Hey, " + name.into<string>() + ".",   // fallback (default locale)
);
```

The runtime's `say_localized` implementation:

1. Looks up `"a3f7c012"` in the active locale's string table.
2. If found, substitutes interpolation slots and displays the translation.
3. If not found, uses the fallback text.

### 28.5 Runtime Functions

The runtime must provide these core functions in the `Runtime` namespace. Dialogue functions are **transition points** —
they suspend execution until the host responds (see §13.9, IL spec §1.14.2).

| Function             | Signature                                                          | Behavior                                                                      |
|----------------------|--------------------------------------------------------------------|-------------------------------------------------------------------------------|
| `say`                | `fn say(speaker: Entity, text: string)`                            | Display text, **suspend** until player advances                               |
| `say_localized`      | `fn say_localized(speaker: Entity, key: string, fallback: string)` | Localized display with string table lookup, **suspend** until player advances |
| `choice`             | `fn choice(options: List<...>) -> int`                             | Present choices, **suspend** until player selects, return selected index      |
| `Entity.getOrCreate` | `fn getOrCreate<T>() -> T`                                         | Get or create singleton entity instance                                       |
| `Entity.findAll`     | `fn findAll<T>() -> EntityList<T>`                                 | Find all entities of a type                                                   |
| `Entity.destroy`     | `fn destroy(entity: Entity)`                                       | Destroy entity, fire `on destroy`, mark dead                                  |
| `Entity.isAlive`     | `fn isAlive(entity: Entity) -> bool`                               | Check if handle refers to a live entity                                       |

---

# Writ IL Specification

**Draft v0.1** — February 2026

---

The intermediate language specification for the Writ virtual machine. Defines the register-based IL design,
instruction set, binary module format, and execution model.

Architectural choices that govern the entire IL design are documented in sections 1.1–1.17. The instruction
set reference follows in sections 2.0–2.16. Instruction encoding tables and opcode assignments are in
sections 3.0–3.2.

---

## 2.1 Register-Based Virtual Machine

**Decision:** Register-based (not stack-based).

**Rationale:**

- All execution state is explicit: each call frame = `(method_id, pc, registers[])`.
- Serialization is straightforward — walk the stack, dump each frame's registers + pc.
- Virtual registers (unlimited per function, numbered sequentially) avoid the complexity of physical register
  allocation. The compiler assigns registers linearly.
- Better fit for JIT compilation (closer to machine register model).

**Implications:**

- Instructions encode source/destination registers explicitly.
- Arguments to calls occupy consecutive registers (compiler arranges this).
- Each function declares its register count in the method body header.

## 2.2 Typed IL with Generic Preservation

**Decision:** The IL preserves full type information. Generics are not monomorphized at compile time.

**Rationale:**

- Enables runtime reflection (planned future feature).
- JIT can monomorphize hot paths selectively.
- Matches CLR model: open generic types in metadata, closed instantiations via TypeSpec.

**Implications:**

- Every register slot has a type identity (known from the method's local type table or inferred from the instruction).
- The runtime carries type tags for dynamic dispatch.
- Metadata tables must represent generic parameters, constraints, and instantiations.
- `(type_tag, contract_id, method_slot) → code_offset` dispatch tables.

## 2.3 Execution Model: Cooperative Yielding with Preemptive Serialization

**Decision:** Functions are normal imperative code in the IL. The runtime manages suspension transparently.

**How it works:**

- The runtime interprets IL instruction-by-instruction on a managed call stack (not the native stack).
- At **transition points** — calls to `say()`, `choice()`, `wait()`, extern calls, `spawn`/`join` — the runtime *may*
  suspend execution and yield to the game engine.
- At **any point**, the runtime can snapshot the entire VM state for save/load.
- The IL itself contains no "yield here" instructions. Yielding is a runtime decision.

**What this is NOT:**

- Functions are NOT compiled into state machines.
- There is no async/await transformation.
- There are no explicit coroutine instructions in the IL.

**Implications:**

- The entire VM state must always be serializable: call stacks, register files, heap objects, globals.
- All data in registers must be of serializable types (no raw native pointers in script-visible state).
- Serialization only occurs at transition points (suspend-and-confirm model, §2.14.2). Native handles and GPU state are
  the host's responsibility.

## 2.4 Binary Format

**Decision:** The primary artifact is a binary module format. Text disassembly is a tooling concern, not part of the
spec.

## 2.5 Instruction Encoding

**Decision:**

- Opcodes: **u16** (65536 slots — future-proof).
- Register operands: **u16** (up to 65535 registers per function).
- Table indices: **u32** metadata tokens (§2.16.4). Heap references are raw u32 offsets.
- Byte order: **little-endian** throughout.
- Instructions are **variable-width**, with the opcode determining the operand layout.

**Instruction shapes:**

| Shape  | Layout                    | Size | Used By                                        |
|--------|---------------------------|------|------------------------------------------------|
| `N`    | `op`                      | 2B   | `NOP`, `RET_VOID`, `ATOMIC_BEGIN`, etc.        |
| `R`    | `op r`                    | 4B   | `LOAD_TRUE r`, `CRASH r`, etc.                 |
| `RR`   | `op r r`                  | 6B   | `MOV r r`, `NEG_I r r`, etc.                   |
| `RRR`  | `op r r r`                | 8B   | `ADD_I r r r`, `CMP_EQ_I r r r`, etc.          |
| `RI32` | `op r i32`                | 8B   | `LOAD_STRING r idx`, `LOAD_GLOBAL r idx`, etc. |
| `RI64` | `op r i64`                | 12B  | `LOAD_INT r val`, `LOAD_FLOAT r val`           |
| `CALL` | `op r_dst i32 r_base u16` | 12B  | `CALL`, `CALL_EXTERN`, `SPAWN_TASK`, etc.      |

Instructions that don't fit these shapes use documented per-instruction layouts (e.g., `SWITCH`, `CALL_VIRT`,
`CALL_INDIRECT`).

**Opcode numbering scheme:**

The u16 opcode space is partitioned by category in the high byte. The low byte identifies the instruction within
its category. Each category has 256 slots, providing room for future expansion without renumbering existing
instructions.

| High Byte | Category                                 |
|-----------|------------------------------------------|
| `0x00`    | Meta                                     |
| `0x01`    | Data Movement                            |
| `0x02`    | Integer Arithmetic                       |
| `0x03`    | Float Arithmetic                         |
| `0x04`    | Bitwise & Logical                        |
| `0x05`    | Comparison                               |
| `0x06`    | Control Flow                             |
| `0x07`    | Calls & Delegates                        |
| `0x08`    | Object Model                             |
| `0x09`    | Arrays                                   |
| `0x0A`    | Type Operations (Option / Result / Enum) |
| `0x0B`    | Concurrency                              |
| `0x0C`    | Globals & Atomics                        |
| `0x0D`    | Conversion                               |
| `0x0E`    | Strings                                  |
| `0x0F`    | Boxing                                   |

Within `0x0A`, sub-ranges separate the three groups: Option at `0x0A00`, Result at `0x0A10`, Enum at `0x0A20`.

The full opcode assignment table is in the summary (03-summary.md §Opcode Assignment Table).

## 2.6 Calling Convention

Arguments are placed in **consecutive registers** starting from a base register in the caller's frame. The callee
receives a fresh register file:

- `r0` = first argument (self for methods, first param for free functions)
- `r1` = second argument, etc.
- Registers beyond parameters are used for locals and temporaries.

**self semantics:**

- A method receiving `self` has it as `r0`. `mut self` is the same slot, with a mutability flag in the method's
  metadata.
- A static function (no self) starts params at `r0`.
- There is no separate calling convention for static vs instance methods — static is simply the absence of self in the
  parameter list.

**Return:** The callee's return value is placed in `r_dst` in the caller's frame. For void functions, `r_dst` is
ignored.

## 2.7 Operator Dispatch

Operators on primitive types use dedicated IL instructions (`ADD_I`, `CMP_LT_F`, etc.) — these are the fast path with no
dispatch overhead.

Operators on user-defined types are lowered by the compiler to `CALL_VIRT` through the corresponding contract (`Add`,
`Sub`, `Eq`, `Ord`, `Index`, etc.). The IL does not have separate "overloaded operator" instructions — the contract
dispatch system handles it uniformly.

This is a **compiler concern**, not an IL concern. The compiler knows the types at emit time and selects the appropriate
instruction.

## 2.8 Serialization Critical Sections — REMOVED

The original design proposed `CRITICAL_BEGIN` / `CRITICAL_END` instructions to mark code regions where serialization
must not occur. This is no longer necessary: the **suspend-and-confirm model** (§2.14.2) ensures the runtime only
serializes at well-defined transition points (host call boundaries, yield points). Since the VM is never serialized
mid-instruction or mid-expression, there is no need for explicit critical sections.

Native resources (OS handles, GPU state) are the host's responsibility and are never included in script saves.

## 2.9 Memory Model

**Decision:** The IL and language assume a **garbage-collected runtime**. The spec does not mandate a specific GC
algorithm (generational, tracing, etc.) — runtime implementors choose — but language semantics are designed for GC and
do not expose manual memory management.

### 2.9.1 Value Types vs Reference Types

| Type                   | Kind                     | Storage                                  | Assignment                     | GC Traced                           |
|------------------------|--------------------------|------------------------------------------|--------------------------------|-------------------------------------|
| `int`, `float`, `bool` | **Value**                | Register (direct bits)                   | Copy bits                      | No                                  |
| `string`               | **Reference, immutable** | Heap (GC-managed)                        | Copy reference                 | Yes                                 |
| Structs                | **Reference**            | Heap (GC-managed)                        | Copy reference (shared object) | Yes                                 |
| Enums                  | **Value**                | Register/stack (tag + inline payload)    | Copy tag + payload             | Payload fields traced if references |
| Arrays                 | **Reference**            | Heap (GC-managed)                        | Copy reference (shared)        | Yes                                 |
| Entities               | **Reference (handle)**   | Entity runtime + GC heap                 | Copy handle                    | Yes                                 |
| Components             | **Extern (host-owned)**  | Host-managed, accessed via entity handle | Via entity reference           | Host responsibility                 |
| Closures/Delegates     | **Reference**            | Heap (GC-managed)                        | Copy reference                 | Yes                                 |

**Enum value semantics:** Enums are value types with inline payloads. The tag is a small integer. Payload fields are
stored inline (for value types) or as references (for reference-typed fields). `Option<int>` is just a tag + an int — no
heap allocation. `Option<string>` is a tag + a string reference. Assignment copies the tag + all payload
bits/references.

### 2.9.2 Assignment and Mutability

`let` / `let mut` controls **binding mutability**, not object mutability:

- `let a = thing` — immutable binding. Cannot reassign `a`. Cannot mutate fields through `a`.
- `let mut a = thing` — mutable binding. Can reassign `a`. Can mutate fields through `a`.

For reference types, assignment copies the reference. Both bindings point to the same object:

```
let mut a = Merchant(name: "Tim", gold: 100);
let mut b = a;     // b and a point to the same object
b.gold += 50;      // a.gold is ALSO now 150
```

This is standard GC-language behavior (Java classes, C# classes, Lua tables).

### 2.9.3 Closure Captures

**Immutable captures (`let`):** The value is copied into the capture struct. For value types, this is a bit copy. For
reference types, this copies the reference (closure and outer scope share the same object, but neither can reassign the
binding).

**Mutable captures (`let mut`):** The compiler generates a **shared capture struct** on the heap that holds the mutable
variable. Both the outer scope and the closure hold a reference to this same struct. The outer scope is rewritten to
access the variable through the struct. Mutations through either side are visible to both.

```
// Source:
let mut count = 0;
let process = fn(x: int) -> int {
    count += 1;    // mutates shared capture
    x + count
};
process(10);       // count is now 1
log(count);        // also 1 — same struct

// Compiler rewrites to (conceptually):
let __env = __closure_env_0 { count: 0 };
let process = Delegate(__closure_body_0, __env);
// process(10) calls __closure_body_0 with __env as first arg
__env.count;       // outer scope accesses through the struct too
```

The capture struct is a compiler-generated type, not a special runtime type:

```
struct __closure_env_0 {
    count: int,   // shared mutable field
}
```

No special runtime types are needed — this is purely a compiler transformation using standard structs.

### 2.9.4 String Handling

- **Literals:** Stored in the module's string heap. Shared, interned. Zero GC pressure at runtime.
- **Runtime strings** (concatenation, format strings, `Into<string>` results): Heap-allocated, GC-managed, not interned.
- **Comparison:** Always by value (character content), regardless of interning. `CMP_EQ_S` compares content.
- **Immutable:** All strings are immutable. Operations like concatenation produce new strings.

### 2.9.5 Entity Lifecycle

Entities have **dual reachability** — they exist in the entity runtime's registry AND as GC-managed objects:

- **Alive:** In the entity registry. Handle is valid. Fields readable/writable.
- **Destroyed:** `DESTROY_ENTITY` called. `on_destroy` fires, defer handlers run, entity removed from registry. Handle
  becomes **dead**.
- **Collected:** GC reclaims memory once the entity is both destroyed AND unreachable from any GC root.

Accessing a dead entity handle (reading fields, calling methods, component access) is a **crash** — same severity as
unwrapping None. Use `Entity.isAlive(entity)` (`ENTITY_IS_ALIVE` instruction) to check liveness without crashing.

Entity handles are opaque runtime-managed identifiers, not direct GC pointers. The runtime resolves handles against its
entity registry. Destruction marks the registry entry as dead but does not invalidate or null existing handles — they
remain valid values that can be stored, passed, and compared. The GC manages the handle objects; an entity's memory is
only collected after it is destroyed AND unreachable from all GC roots.

`Entity.getOrCreate<T>()` for a destroyed singleton **recreates it** (the semantics are get-or-*create*).

**Runtime guidance (entity storage):** Entities should be stored in a registry keyed by opaque handle IDs — a
generation-indexed array is recommended, where each slot holds the entity's script state and a generation counter for
handle validation (stale handles detected by generation mismatch). Component access via `GET_COMPONENT` should resolve
against the entity's `ComponentSlot` list from the TypeDef metadata — a type-tag lookup returning the component
reference or `None`. Singleton entities (marked `[Singleton]`) should be maintained in a per-type registry indexed by
TypeDef token; `GET_OR_CREATE` checks this registry first and falls through to full entity construction if absent.

### 2.9.6 GC Roots

The GC traces from these roots:

1. **All registers in all active task call stacks** — the IL type metadata tells the GC exactly which registers hold
   references at any PC (precise scanning, not conservative).
2. **All global variables** — `global mut` and `const` holding reference types.
3. **The entity registry** — all live (non-destroyed) entities.
4. **The task handle tree** — handles to spawned tasks.

### 2.9.7 Garbage Collection

The spec assumes garbage collection but does not prescribe a specific algorithm. The typed register model (§2.15.1)
provides complete type information for every register at every program point, enabling **precise root scanning** — the
runtime should use this to identify GC references exactly rather than conservatively scanning memory. A generational or
incremental collector is recommended for game workloads to minimize stop-the-world pause times.

**Finalization ordering:** When multiple unreachable objects are collected in the same GC cycle, the order in which
`on finalize` hooks execute is implementation-defined. Finalizers must not assume that other managed objects referenced
by the finalizing object are in a valid state or have not yet been finalized. Deterministic cleanup logic belongs in
`on destroy` (for entities) or application-level teardown, not in finalizers.

**Finalization execution:** Finalizer hooks should be queued during GC tracing and executed as tasks during a subsequent
scheduling pass — not during the GC pause itself. This allows finalizer code to execute normally, including suspension
at transition points.

### 2.9.8 IL Implications

- `MOV` copies register contents. For references, this copies the pointer — no deep copy, no clone.
- No `FREE` / `DEALLOC` instructions exist. The GC handles all reclamation.
- `NEW`, `NEW_ARRAY`, `NEW_ENUM`, `SPAWN_ENTITY` are allocation points. The GC may trigger during any allocation.
- **GC safepoints** are a runtime concern, not an IL concern. The runtime can GC at any instruction boundary because
  type metadata enables precise root scanning.
- Dead entity access requires a liveness check in the runtime on field/method access through entity handles.

## 2.10 Self Parameter

**Decision:** Methods take explicit `self` or `mut self` as their first parameter. This is now specified in the language
spec (§12.5).

- `self` — immutable receiver. Cannot modify fields or call `mut self` methods through `self`.
- `mut self` — mutable receiver. Can read and modify fields, call any method through `self`.
- Absence of `self` — static function (no receiver).

**IL mapping:**

- `self` is always `r0` in the callee's register file (see §2.6).
- The method's metadata carries a mutability flag: `is_mut_self: bool`.
- The runtime enforces that `mut self` methods are only called through mutable bindings (or the compiler enforces this
  statically — either is valid).
- Operator methods have implicit `self` with mutability determined by operator kind: all read operators are immutable,
  `[]=` is mutable.
- Lifecycle hooks (`on create`, `on interact`, `on destroy`) have implicit `mut self`.

## 2.11 Construction Model

**Decision:** Construction uses the `new` keyword with brace-syntax for all types. No user-defined constructors.
`spawn` is reserved for task concurrency only.

**Syntax:** `new Type { field: value, ... }` for both structs and entities. The `new` keyword disambiguates
construction from block expressions, making the syntax unambiguous for the parser. The compiler determines the IL
sequence from the type's kind.

**Default field values:** Defaults can be runtime expressions (e.g., `List::new()`). The compiler inlines the default
expression at every construction site that doesn't override the field. `NEW` allocates zeroed memory — the compiler
emits explicit code for all field initialization.

**Struct construction:**

1. `NEW type_idx` — allocate zeroed memory.
2. `SET_FIELD` / `LOAD_*` for every field (defaults + overrides).
3. `CALL __on_create` — run the `on create` hook body, if defined.

**Entity construction:**

1. `SPAWN_ENTITY type_token` — allocate entity. Set the entity's internal "under construction" flag. Notify the host
   with the component list (from ComponentSlot metadata) so it can prepare native representations.
2. `SET_FIELD` for script fields (written to heap directly) and component fields (**buffered**, not sent to host).
3. `INIT_ENTITY` — flush all buffered component field values to the host as a single batch. Clear the "under
   construction" flag. Fire the `on_create` lifecycle hook.

The separation of SPAWN_ENTITY and INIT_ENTITY ensures field overrides are visible inside `on_create`. Component field
buffering avoids per-field round-trips through suspend-and-confirm during construction. See §2.16.7 for the full
buffering specification and safety invariants.

**No constructors:** Construction is entirely compiler-generated. `new Type { ... }` produces `NEW`/`SPAWN_ENTITY` +
`SET_FIELD` + `on_create`. Fields without defaults are required at every construction site. For convenience factories,
use static methods: `Merchant::create("Tim")`.

**Lifecycle hooks:** Both structs and entities support lifecycle hooks (`on create`, `on finalize`, `on serialize`,
`on deserialize`). Entities additionally support `on destroy` and `on interact`. All hooks receive implicit `mut self`.
Hooks lower to regular methods stored in the TypeDef metadata.

## 2.12 Delegate Model (Closures & Function Values)

All function values in Writ — named function references, closures, and bound method references — are **delegates**. This
borrows from the C# delegate model.

### 2.12.1 Delegate Structure

A delegate is a GC-managed object containing:

```
Delegate {
    target: Option<object>,   // capture struct, self, or null
    method: method_index,     // resolved concrete method
}
```

### 2.12.2 Creation Scenarios

**Plain function reference** (no target):

```
// let func = add;  (IL supports this; language syntax TBD due to overloading)
NEW_DELEGATE  r_func, method_idx(add), r_null    // target = null
```

**Closure with captures:**

```
// let f = fn(x: int) -> int { x + bonus };
NEW           r_env, __closure_env_type            // capture struct
SET_FIELD     r_env, bonus_field, r_bonus          // copy captured value
NEW_DELEGATE  r_f, method_idx(__closure_body), r_env  // target = capture struct
```

**Closure without captures** (optimized — no allocation for empty env):

```
// let f = fn(x: int) -> int { x + 1 };
NEW_DELEGATE  r_f, method_idx(__closure_body), r_null   // target = null, no env needed
```

**Bound method reference:**

```
// let greet = merchant.greet;
NEW_DELEGATE  r_greet, method_idx(Merchant::greet), r_merchant  // target = self
```

### 2.12.3 Invocation

All delegates are called with `CALL_INDIRECT`:

```
CALL_INDIRECT  r_result, r_delegate, r_base, argc
```

The runtime:

1. Reads the delegate from `r_delegate`.
2. Extracts `target` and `method`.
3. If `target` is non-null, prepends it as the first argument (it becomes `r0` / self / env in the callee).
4. Calls the resolved method.

The callee does not know or care whether it was called directly, through a delegate, or through a closure.

### 2.12.4 Virtual Method References

`NEW_DELEGATE` always takes a **concrete method index**. For virtual/contract methods, the compiler resolves the
dispatch at delegate creation time. If that's not possible (rare), the compiler generates a small wrapper closure that
performs the virtual call internally.

### 2.12.5 Relationship to Function Types

The language spec's `fn(int, int) -> int` type corresponds to a delegate in the IL. Every value of a function type is a
delegate. No language spec change is needed beyond documenting this representation.

## 2.13 Save/Load Serialization

**Status:** Resolved.

The runtime must be able to serialize and deserialize the entire VM state for game save/load. The host decides **when**
to serialize or deserialize — this is not a runtime concern. The spec defines **what** must be serializable and *
*recommends** strategies for version compatibility, but the save format and migration policies are runtime concerns.

### 2.13.1 Spec Requirements

The runtime **must** support serializing and restoring the full VM state. The following state constitutes a complete
save:

- All task call stacks (frames: method_id, pc, register values)
- All global variable values
- The full heap (all live GC objects: structs, arrays, strings, closures, delegates)
- The entity registry (all live entities with their script-side field values)
- The task tree (parent-child relationships, scoped vs detached)

The following is explicitly **excluded** from the script save:

- Native/host state (sprites, physics bodies, audio) — the host is responsible for its own save/load
- Extern component field values — these are host-owned (see §2.14)

The runtime must not attempt to serialize while any extern call is in-flight. The suspend-and-confirm model (§2.14.2)
guarantees that the VM is at a well-defined transition point before the host can request a save, but the runtime must
additionally ensure all pending host confirmations have resolved before serializing.

### 2.13.2 Module Versioning

Each compiled IL module carries a **version identifier** (format is runtime-defined — content hash, semantic version, or
both). On deserialization, the runtime compares the saved module version against the currently loaded module.

If a version mismatch is detected, the runtime **must** report the conflict to the host. Behavior beyond reporting is
runtime-defined, but the spec **recommends** the following strategy:

**Recommended: IL coexistence.** The save includes the full IL module binary that was active at save time. On restore,
if the current IL differs, the runtime loads the saved IL alongside the current IL. Existing call stacks continue
executing against the saved (old) IL. As stack frames return, they re-enter the current (new) IL at function call
boundaries. The old IL is discarded once no call stacks reference it.

This approach handles the common case — a mod update or patch between play sessions — without requiring migration logic.
The old code naturally drains out as functions return.

**Limitations the runtime should be aware of:**

- Code that never returns (e.g., `while true { ... }` with no function calls in the body) will run the old IL
  indefinitely. The compiler may warn about such patterns and recommend inserting function call boundaries inside
  long-running loops.
- Game authors building moddable games should provide extern mechanisms like timers or event hooks that allow looping
  behavior to pass through function call boundaries, enabling IL transitions.

### 2.13.3 Extern Calls During Serialization

Serialization must not occur while extern calls are outstanding. Since the suspend-and-confirm model suspends the VM on
extern calls until the host confirms, a well-behaved host will not request a save while it has unconfirmed operations
pending. If it does, the runtime must either defer the save until all pending extern calls resolve, or reject the save
request.

## 2.14 Runtime-Host Interface

**Status:** Resolved.

The Writ runtime is embedded inside a host game engine. The runtime owns script state (IL execution, GC, entity
registry) but depends on the host for native capabilities (rendering, physics, audio, input). This interface defines the
contract between the two.

### 2.14.1 Architecture

```
+-----------------------------------------------------------+
|  Host Engine (Godot, Unity, custom, etc.)                  |
|    - Rendering (Sprite)          - Physics (Collider)      |
|    - Audio                       - Input                   |
|    - Native entity storage       - Platform services       |
+----------------------- Host API --------------------------+
|  Writ Runtime                                              |
|    - IL interpreter              - GC                      |
|    - Script state (heap)         - Task scheduler          |
|    - Entity registry (script)    - Contract dispatch       |
+-----------------------------------------------------------+
```

### 2.14.2 Runtime -> Host (requests — runtime suspends until host confirms)

The runtime does not fire-and-forget notifications. When the runtime needs the host to perform an action, it **suspends
execution** until the host has processed the request and confirmed the result. This ensures consistency with the game
engine's logic loop — the host processes changes on its own tick, not asynchronously.

| Request                   | Data Provided                                           | Host Responsibility                                                                           |
|---------------------------|---------------------------------------------------------|-----------------------------------------------------------------------------------------------|
| **Entity spawned**        | type info, initial field values, component field values | Create native representation (sprite, physics body, etc.). Confirm when ready.                |
| **Entity destroyed**      | entity handle                                           | Clean up native resources. Confirm when done.                                                 |
| **Entity unreferenced**   | entity handle, is_singleton                             | Script has no more references. Host decides whether to keep or destroy native side.           |
| **Component field write** | entity handle, component type, field id, new value      | Update native state (e.g., `sprite.visible = false`). Confirm when applied.                   |
| **Component field read**  | entity handle, component type, field id                 | Return current native value to the runtime.                                                   |
| **Extern function call**  | extern index, arguments                                 | Execute native implementation, return result.                                                 |
| **Say / choice / wait**   | speaker, text/options, duration                         | Display dialogue, present choices, wait for time/input.                                       |
| **Save requested**        | —                                                       | Runtime is about to serialize. Host should prepare (flush buffers, etc.). Confirm when ready. |

### 2.14.3 Host -> Runtime (commands the host sends)

| Command            | Data                                                      | Purpose                                           |
|--------------------|-----------------------------------------------------------|---------------------------------------------------|
| **Tick**           | delta time                                                | Advance script execution (resume suspended tasks) |
| **Fire event**     | entity handle, event type (interact/create/destroy), args | Trigger entity lifecycle hooks                    |
| **Start dialogue** | dlg method index, arguments                               | Begin a dialogue sequence                         |
| **Request save**   | —                                                         | Ask the runtime to serialize VM state             |
| **Load save**      | save data                                                 | Restore VM state from a save                      |
| **Hot reload**     | new IL module                                             | Replace running scripts (if supported)            |

### 2.14.4 Entity Ownership Model

**Decision:** The runtime owns all script-defined state. The host owns all native state.

- **Script fields** (fields declared in `entity { ... }`): Stored in the runtime's GC heap. Accessed via `GET_FIELD` /
  `SET_FIELD` directly.
- **Components** (always extern, e.g., `extern component Sprite { ... }`): Data-only schemas. Storage and implementation
  are host-provided. `GET_FIELD` / `SET_FIELD` on component fields are proxied through the host API. Components have no
  script-defined methods.

This means:

- `SPAWN_ENTITY` -> runtime allocates entity in its heap, then notifies host with component initial values.
- `SET_FIELD` on script field -> runtime updates heap directly.
- `SET_FIELD` on component field -> runtime proxies to host, suspends until host confirms.
- `GET_COMPONENT` -> runtime proxies to host.
- `DESTROY_ENTITY` -> runtime fires `on_destroy`, runs defers, removes from registry, notifies host.

### 2.14.5 Singleton Entities and the Host

`[Singleton]` entities have special semantics at the runtime-host boundary:

- When `getOrCreate<T>()` first creates a singleton, the runtime notifies the host so it can create the native
  representation.
- The host may pre-register a native entity that should bind to a `[Singleton]` type. On `getOrCreate`, the runtime
  binds to the existing native entity instead of asking the host to create a new one.
- When the runtime notifies the host of "entity unreferenced," the `is_singleton` flag tells the host this entity should
  likely be preserved (singletons are expected to exist for the lifetime of the game).

### 2.14.6 Scripted Entities: Runtime Requirements

The spec allows entities to be defined entirely in Writ scripts. The runtime MUST support these — it cannot refuse
script-defined entities. However, the runtime is not required to provide a rendering/physics implementation for them. A
script-only entity with no extern components is purely script state.

If a scripted entity uses extern components (`use Sprite { ... }`), the host MUST provide those components. If the host
does not support a required extern component, entity spawning fails with a crash (same semantics as a failed library
load — unrecoverable, defer unwinding).

### 2.14.7 Runtime Logging Interface

The runtime must provide a logging interface that reports events to the host with a severity level. The host decides
how to handle log messages (display to user, write to file, ignore, etc.).

**Required log levels:** `error`, `warn`, `info`, `debug`.

**Events the runtime must log:**

| Event                | Level   | Description                                                                                                                                                           |
|----------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Lifecycle hook crash | `error` | A lifecycle hook (`on create`, `on destroy`, `on serialize`, `on deserialize`, `on finalize`) crashed. Includes the entity/struct type, hook name, and error details. |
| Task crash           | `error` | A task's call stack unwound due to an unhandled crash (`!` on None/Err, out-of-bounds, etc.).                                                                         |
| Entity unreferenced  | `debug` | GC determined no script references remain for an entity.                                                                                                              |
| Version mismatch     | `warn`  | IL module version differs between save and current load (see §2.13.2).                                                                                                |

The logging interface is the primary mechanism for the runtime to communicate errors to the host. The spec does not
prescribe the format — the runtime may use callbacks, a message queue, or any other mechanism appropriate to the
embedding environment.

### 2.14.8 Implementation Guidance

The following are **recommendations**, not requirements. Runtime implementors may deviate based on their host
environment.

- **Protocol format:** The runtime-host interface may be implemented as direct function calls (C FFI), a message queue,
  or any other IPC mechanism. For single-process embeddings (the common case), direct function calls with callback
  registration are simplest.
- **Extern function errors:** Extern functions that can fail should return `Result<T, E>` at the Writ level. The
  runtime should not silently swallow host-side errors — propagate them as Writ `Result::Err` values.
- **Hot reload:** If supported, the runtime should apply IL coexistence (§2.13.2) — existing call stacks continue on
  old IL, new calls use new IL. This is the same mechanism as save/load version mismatch handling.
- **Component field validation:** If the host rejects a component field write (invalid value, read-only field), the
  runtime should crash the calling task with a descriptive error, logged via §2.14.7.
- **Dialogue functions:** `say`, `choice`, and `wait` are transition points that suspend execution. The host is
  responsible for presenting UI and signaling completion. See spec §13.9 for the language-level semantics.

## 2.15 IL Type System

The IL preserves the full Writ type system in metadata. Types are not erased — the runtime has access to complete type
information for dispatch, serialization, and reflection.

**Open items:**

- **Register sizing:** Whether registers are fixed-width or variable-width is deferred to D1.
- **Concrete encoding examples:** To be added in a future pass once instruction encoding (D1) is finalized.

**Resolved in §2.16:** The well-known type table proposed in the earlier B1 TODO is no longer needed. Core types
(Option, Result, etc.) are provided by the `writ-runtime` module (§2.16.8) and referenced via standard cross-module
TypeRef resolution. The blob heap format, register type table, and all metadata tables are specified in §2.16.

### 2.15.1 Register Model

IL functions operate on a set of **typed registers**. Each register holds exactly one value of its declared type. The
compiler emits type declarations for all registers in the method body metadata (see B3 in il-todo.md).

- For **value types** (int, float, bool, enums), the register holds the value directly.
- For **reference types** (string, structs, entities, arrays, closures/delegates), the register holds a GC-managed
  reference.

Registers are abstract — the spec does not mandate a physical size or layout. The runtime uses the register's declared
type to determine storage requirements. A `MOV` instruction copies the full value regardless of the underlying type's
physical size.

This means the IL does not concern itself with "how many bytes is an enum register." The compiler declares
`r3: QuestStatus`,
the runtime allocates whatever storage it needs for that type, and instructions like `NEW_ENUM`, `GET_TAG`, and
`EXTRACT_FIELD` operate on that register as a single unit.

### 2.15.2 Primitive Type Tags

Primitives have fixed type tags in the type reference encoding:

| Tag (u8) | Type     | Kind      | Register Contents                              |
|----------|----------|-----------|------------------------------------------------|
| `0x00`   | `void`   | —         | Zero-width, return-only                        |
| `0x01`   | `int`    | Value     | 64-bit signed integer                          |
| `0x02`   | `float`  | Value     | 64-bit IEEE 754                                |
| `0x03`   | `bool`   | Value     | Logical 0/1                                    |
| `0x04`   | `string` | Reference | GC pointer to heap-allocated, immutable string |

`bool` occupies a full register slot at runtime even though it is logically 1 bit. The spec does not mandate
bit-packing.

### 2.15.3 Type Reference Encoding

A **TypeRef** is a variable-length encoded type descriptor stored in the blob heap. TypeRefs appear wherever the
metadata references a type: field types, parameter types, return types, register type declarations, generic arguments.

| Kind (u8)     | Payload                      | Meaning                                                                                                               |
|---------------|------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `0x00`–`0x04` | —                            | Primitive (void, int, float, bool, string)                                                                            |
| `0x10`        | TypeDef index (`u32`)        | Named type — struct, enum, entity, or component. The TypeDef entry carries a `kind` flag distinguishing these.        |
| `0x11`        | TypeSpec index (`u32`)       | Instantiated generic type (e.g., `List<int>`, `Option<Guard>`)                                                        |
| `0x12`        | GenericParam ordinal (`u16`) | Open type parameter — the Nth generic param on the enclosing TypeDef or MethodDef                                     |
| `0x20`        | element TypeRef              | `Array<T>` — recursive encoding. The element is itself a TypeRef.                                                     |
| `0x30`        | blob offset (`u32`)          | Function/delegate type — points to a signature blob: `param_count: u16, param_types: TypeRef[], return_type: TypeRef` |

**Design notes:**

- **Single TypeDef table.** All named types (structs, enums, entities, components) share one TypeDef table. The TypeDef
  entry's `kind` field distinguishes them. TypeRefs do not encode the kind — it is looked up from the TypeDef.
- **Option and Result are regular generic enums** in the type system. `Option<int>` is represented as a TypeSpec entry
  pointing to the `Option` TypeDef with type argument `int`. Their specialness exists only at the instruction level
  (`WRAP_SOME`, `IS_OK`, etc.), not in the type encoding.
- **Closure/delegate types.** A closure is a compiler-generated TypeDef (per §2.12). Its TypeRef is a `0x10` pointing
  to that generated TypeDef. The callable signature is encoded separately in the delegate metadata.
- **Recursive encoding.** TypeRefs nest: `Array<Option<int>>` encodes as `0x20` → `0x11` →
  TypeSpec(Option_TypeDef, [`0x01`]).

### 2.15.4 Generic Representation

**In metadata:**

- **Open generic types:** A TypeDef may have one or more `GenericParam` rows, each with a zero-based ordinal.
  `List<T>` has one GenericParam (ordinal 0). `Map<K, V>` has two (ordinals 0, 1).
- **Generic constraints:** `GenericConstraint` rows bind a GenericParam to required contracts. `T: Add + Eq` produces
  two constraint rows, each referencing the GenericParam and a contract TypeDef.
- **Instantiated types:** A `TypeSpec` entry references a TypeDef plus a list of concrete TypeRef arguments.
  `List<int>` = TypeSpec(List_TypeDef, [`0x01`]). `Map<string, int>` = TypeSpec(Map_TypeDef, [`0x04`, `0x01`]).
- **Generic methods:** Same mechanism — GenericParam rows are attached to the MethodDef instead of the TypeDef.
  Call sites provide type arguments in the `CALL` instruction's metadata.

**At runtime (generic dispatch):**

The spec requires that generic code executes correctly but does not mandate a specific dispatch mechanism. The
conceptual model:

When IL code calls a method on a generic type parameter `T` (e.g., `value.method()` where `value: T`), the runtime:

1. Determines the **concrete type tag** of the value. For reference types, the tag is stored on the heap object header.
   For value types passed through generic parameters, the value is **boxed** (see below).
2. Resolves the method via the **contract dispatch table**: a mapping from
   `(concrete_type_tag, contract_id, method_slot)` to a method entry point.
3. Calls the resolved method.

**Boxing:** When a value type (`int`, `float`, `bool`, enum) is passed to a generic parameter, the runtime boxes it —
allocating a small heap object that wraps the value and carries a type tag. This allows uniform representation through
generic code paths. The runtime may unbox or avoid boxing when the concrete type is statically known, but the spec does
not require such optimizations.

Runtimes may use any dispatch implementation: hash tables, vtable-style arrays, polymorphic inline caches, or
monomorphization of hot paths. The spec mandates correct behavior, not a specific strategy.

**Runtime guidance:** The runtime should build dispatch structures from `ImplDef` rows at module load time. Each
`ImplDef` maps a `(type, contract)` pair to a method list; the runtime should flatten these into a lookup structure
indexed by `(concrete_type_tag, contract_id, method_slot)` for O(1) dispatch. A flat table or hash map is the
recommended approach for predictable performance. Polymorphic inline caching at `CALL_VIRT` sites is a permitted
optimization for hot call sites.

### 2.15.5 Enum Representation

Enums are value types. An enum value consists of a **tag** and an optional **payload**.

**Tag:** A `u16` discriminant identifying the active variant. Supports up to 65535 variants per enum type.

**Layout by variant kind:**

- **Tag-only variants** (e.g., `QuestStatus::NotStarted`): Only the tag. No payload space.
- **Payload variants** (e.g., `QuestStatus::InProgress(step: int)`): Tag + inline payload fields, laid out
  consecutively per the variant's field definitions in the TypeDef.

**Total size:** An enum value's size is `sizeof(tag) + sizeof(largest_variant_payload)` across all variants. All
variants occupy the same total space; smaller variants are padded. The compiler calculates the layout from the TypeDef
at emit time.

**In registers:** An enum register holds the complete enum value (tag + payload) as a single abstract unit. The
runtime determines the physical storage from the register's declared type (see §2.15.1).

**Payload field types:** Payload fields follow the same rules as struct fields. Value-typed payload fields are stored
inline. Reference-typed payload fields store GC references and are traced by the garbage collector.

**Examples:**

```
enum QuestStatus {
    NotStarted,                    // tag-only: tag=0, payload=0 bytes
    InProgress(currentStep: int),  // tag=1, payload=8 bytes (one int)
    Completed,                     // tag-only: tag=2, payload=0 bytes
    Failed(reason: string),        // tag=3, payload=ref-size (one string ref)
}
// Total size: sizeof(u16) + max(0, 8, 0, ref-size) = tag + 8 bytes
```

**Option\<T\> null-pointer optimization:** When `T` is a non-nullable reference type, a runtime *may* represent
`Option<T>` as a bare reference where `null` = `None` and non-null = `Some(value)`. This is a permitted runtime
optimization, not mandated by the spec. IL code uses `WRAP_SOME` / `IS_NONE` / etc. regardless — the runtime may
elide them internally.

## 2.16 IL Module Format

The compiled IL is stored in a binary module format. Each module is a self-contained compilation unit that may reference
types and methods from other modules. At load time, the runtime loads all modules into a single domain and resolves
cross-module references by name.

### 2.16.1 Binary Container

**Magic and version:**

```
Bytes 0–3:   0x57 0x52 0x49 0x54  ("WRIT")
Bytes 4–5:   u16 format_version    (starts at 1, bumps on incompatible layout changes)
Bytes 6–7:   u16 flags             (bit 0 = debug info present, rest reserved)
```

**Module header** (fixed layout, immediately after the magic):

```
module_name:        u32   // string heap offset
module_version:     u32   // string heap offset — semver string (§2.16.3)

string_heap_offset: u32
string_heap_size:   u32
blob_heap_offset:   u32
blob_heap_size:     u32

table_directory:    [offset: u32, row_count: u32] × 21
```

The table directory has a fixed-order entry for each of the 21 metadata tables (§2.16.5). Empty tables have
`row_count = 0`. Total header size: 8 (magic/version/flags) + 8 (name/version) + 16 (heaps) + 168 (table directory)
= **200 bytes**.

**String Heap:** Length-prefixed UTF-8. Each entry is `u32(byte_length)` followed by the string bytes. No null
terminator. Offset 0 is reserved as the empty/null string.

**Blob Heap:** Same encoding as the string heap (length-prefixed byte sequences). Stores TypeRef encodings (§2.15.3),
method signatures, constant values, and component override data.

**Byte order:** Little-endian throughout (§2.5). Table rows are aligned to 4-byte boundaries.

### 2.16.2 Multi-Module Architecture

Modules may depend on other modules. Dependencies are declared in the **ModuleRef** table and must form a directed
acyclic graph (DAG) — circular dependencies are forbidden. The runtime loads all modules into a single domain and
resolves cross-module references at load time.

**Cross-module references** are name-based:

- **TypeRef** rows reference a type in another module by `(ModuleRef, namespace, name)`. At load time, the runtime
  resolves each TypeRef to a TypeDef in the target module.
- **MethodRef** rows reference a method by `(parent type, name, signature)`. Resolved to a MethodDef at load time.
- **FieldRef** rows reference a field by `(parent type, name, type signature)`. Resolved to a FieldDef at load time.
  This provides ABI-safe cross-module field access — recompiling a dependency that reorders fields does not break
  dependent modules as long as field names and types are preserved.

After load-time resolution, cross-module references are equivalent to direct local references. The resolution cost is
paid once at load time.

**Compilation model:** The compiler needs access to referenced modules' metadata at compile time to emit correct
TypeRef, MethodRef, and FieldRef entries. This is analogous to include paths — the build system provides paths to
dependency modules, and the compiler reads their metadata tables.

### 2.16.3 Module Versioning

Each module declares its version as a **Semantic Versioning 3.0.0** (semver) string in the format `MAJOR.MINOR.PATCH`:

- **MAJOR** — incremented for breaking changes (removed types, changed signatures, incompatible behavior).
- **MINOR** — incremented for backwards-compatible additions (new types, new methods, new fields with defaults).
- **PATCH** — incremented for backwards-compatible bug fixes.

Semantic Versioning is a widely adopted convention that encodes compatibility information in a version number. The key
principle is that consumers can safely upgrade within the same major version. A change from `2.2.0` to `2.3.0` is safe
(new features, nothing removed). A change from `1.x` to `3.0.0` signals breaking changes that require consumer updates.

**Compatibility rule:** A loaded module with version `A.B.C` satisfies a dependency requirement of `>=X.Y.Z` when
`A == X` and `(A, B, C) >= (X, Y, Z)` by lexicographic comparison. The major version must match exactly (a major
version change signals breaking incompatibility); the minor and patch versions must be equal to or greater than the
requirement.

**ModuleRef** entries include a `min_version` field. At load time, the runtime checks that each dependency's version
satisfies the requirement. On failure, the runtime logs the mismatch (§2.14.7) and may refuse to load or proceed at the
host's discretion.

### 2.16.4 Metadata Tokens

Instructions and metadata entries reference types, methods, and fields via **metadata tokens** — u32 values encoding
both the target table and the row index:

```
Bits 31–24:  table ID (0–20, matching the table directory order in §2.16.5)
Bits 23–0:   row index (1-based; 0 = null token)
```

This gives 24-bit row indices (up to 16,777,215 rows per table per module).

**Examples:**

| Token         | Meaning                                                        |
|---------------|----------------------------------------------------------------|
| `0x02_000005` | TypeDef row 5 (type defined in this module)                    |
| `0x03_000003` | TypeRef row 3 (type in another module — resolved at load time) |
| `0x07_00000A` | MethodDef row 10 (method defined here)                         |
| `0x08_000002` | MethodRef row 2 (method in another module)                     |

After load-time resolution, the runtime may remap cross-module tokens internally. The token encoding is a
storage/interchange format — the runtime's internal representation is implementation-defined.

### 2.16.5 Metadata Tables

All tables have **fixed-size rows**. References to heaps are u32 offsets. References to other tables are metadata tokens
(§2.16.4). Tables use the **list ownership** pattern: a parent's `xxx_list` field gives the index of the first child
row, and the range extends to the next parent's `xxx_list` value (or end of table).

| #  | Table                 | Key Fields                                                                               | Purpose                                               |
|----|-----------------------|------------------------------------------------------------------------------------------|-------------------------------------------------------|
| 0  | **ModuleDef**         | name(str), version(str), flags(u32)                                                      | Module identity (always 1 row)                        |
| 1  | **ModuleRef**         | name(str), min_version(str)                                                              | Dependencies on other modules                         |
| 2  | **TypeDef**           | name(str), namespace(str), kind(u8), flags(u16), field_list, method_list                 | Types defined in this module                          |
| 3  | **TypeRef**           | scope(token:ModuleRef), name(str), namespace(str)                                        | Types in other modules (resolved at load time)        |
| 4  | **TypeSpec**          | signature(blob)                                                                          | Instantiated generic types (TypeDef + type arguments) |
| 5  | **FieldDef**          | name(str), type_sig(blob), flags(u16)                                                    | Fields on types defined here                          |
| 6  | **FieldRef**          | parent(token), name(str), type_sig(blob)                                                 | Fields in other modules (resolved at load time)       |
| 7  | **MethodDef**         | name(str), signature(blob), flags(u16), body_offset(u32), body_size(u32), reg_count(u16) | Methods/functions defined here                        |
| 8  | **MethodRef**         | parent(token), name(str), signature(blob)                                                | Methods in other modules (resolved at load time)      |
| 9  | **ParamDef**          | name(str), type_sig(blob), sequence(u16)                                                 | Method parameters                                     |
| 10 | **ContractDef**       | name(str), namespace(str), method_list, generic_param_list                               | Contract declarations                                 |
| 11 | **ContractMethod**    | name(str), signature(blob), slot(u16)                                                    | Method slots within a contract                        |
| 12 | **ImplDef**           | type(token), contract(token), method_list                                                | Contract implementations                              |
| 13 | **GenericParam**      | owner(token), owner_kind(u8), ordinal(u16), name(str)                                    | Type parameters on types/methods                      |
| 14 | **GenericConstraint** | param(row:GenericParam), constraint(token)                                               | Bounds on type parameters                             |
| 15 | **GlobalDef**         | name(str), type_sig(blob), flags(u16), init_value(blob)                                  | Constants and `global mut` variables                  |
| 16 | **ExternDef**         | name(str), signature(blob), import_name(str), flags(u16)                                 | Extern function/type declarations                     |
| 17 | **ComponentSlot**     | owner_entity(token:TypeDef), component_type(token)                                       | Entity → component bindings                           |
| 18 | **LocaleDef**         | dlg_method(token:MethodDef), locale(str), loc_method(token:MethodDef)                    | Dialogue locale dispatch                              |
| 19 | **ExportDef**         | name(str), item_kind(u8), item(token)                                                    | Convenience index of pub-visible items                |
| 20 | **AttributeDef**      | owner(token), owner_kind(u8), name(str), value(blob)                                     | Metadata attributes ([Singleton], etc.)               |

**TypeDef.kind:** `0 = struct`, `1 = enum`, `2 = entity`, `3 = component`.

**MethodDef.flags** includes: visibility (pub/private), is_static, is_mut_self, hook_kind (0=none, 1=create, 2=destroy,
3=finalize, 4=serialize, 5=deserialize, 6=interact), and an **intrinsic** flag for `writ-runtime` native
implementations (§2.16.8).

**FieldDef.flags** includes: visibility (pub/private), has_default, is_component_field.

### 2.16.6 Method Body Layout

Each method body starts at the MethodDef's `body_offset` and occupies `body_size` bytes:

```
MethodBody {
    register_types: u32[reg_count]    // blob heap offsets — one TypeRef per register
    code_size:      u32
    code:           u8[code_size]     // instruction stream

    // Present only if module flags bit 0 (debug) is set:
    debug_local_count:  u16
    debug_locals:       DebugLocal[debug_local_count]
    source_span_count:  u32
    source_spans:       SourceSpan[source_span_count]
}
```

**Register type table:** `reg_count` (from MethodDef) entries, each a u32 blob heap offset pointing to a TypeRef
encoding (§2.15.3). The runtime reads these at method load to determine per-register storage requirements. Common
TypeRefs are naturally deduplicated in the blob heap.

**Debug info** (optional):

```
DebugLocal  { register: u16, name: u32(str_offset), start_pc: u32, end_pc: u32 }
SourceSpan  { pc: u32, line: u32, column: u16 }
```

No defer table or exception table is needed in the method body. The defer stack is runtime state managed by
`DEFER_PUSH`/`DEFER_POP` instructions. Writ has no try/catch, so no exception handler table.

### 2.16.7 Entity Construction Buffering

During entity construction, component field writes are **buffered** by the runtime and delivered to the host as a single
batch when `INIT_ENTITY` executes. This avoids per-field round-trips through suspend-and-confirm (§2.14.2) during
construction.

**Construction sequence:**

1. `SPAWN_ENTITY r, type_token` — Allocate entity in the runtime's heap. Set the entity's internal "under construction"
   flag. Notify the host with the component list (from the ComponentSlot table) so it can prepare native
   representations.
2. `SET_FIELD r, field_token, r_val` on **script fields** — Written directly to the script heap. No host involvement.
3. `SET_FIELD r, field_token, r_val` on **component fields** — **Buffered** by the runtime. Not sent to host.
4. `INIT_ENTITY r` — Flush all buffered component field values to the host as a single batch. Clear the "under
   construction" flag. Fire the `on_create` lifecycle hook.

**Safety invariant:** Every `SPAWN_ENTITY` must be followed by exactly one `INIT_ENTITY` for the same entity before the
enclosing frame returns. If a frame exits with an entity still in "under construction" state, the runtime crashes the
task and logs the error (§2.14.7). The compiler guarantees this pairing — `INIT_ENTITY` is always emitted as part of
the `new Entity { ... }` lowering.

**After construction:** `SET_FIELD` on component fields goes to the host immediately via suspend-and-confirm (§2.14.2).
Buffering applies only during the SPAWN_ENTITY → INIT_ENTITY construction window.

### 2.16.8 The `writ-runtime` Module

The `writ-runtime` module is a **runtime-provided module** containing core type definitions that the compiler and IL
instructions depend on. Unlike normal modules, `writ-runtime` is not compiled from Writ source — the runtime provides
it as part of its implementation. The spec mandates what types this module must contain and what layouts they must have.
The runtime is free to implement them however it chooses internally.

Methods on `writ-runtime` types may carry an **intrinsic** flag on their MethodDef entries, indicating that the runtime
provides a native implementation rather than IL bytecode. This allows core operations (such as contract implementations
on primitive types) to execute as optimized native code while appearing as normal methods in the metadata for generic
dispatch, reflection, and cross-module referencing.

A separate **`writ-std`** module (a standard library written in Writ) may provide utility types like `List<T>`,
`Map<K, V>`, and common helper functions. Unlike `writ-runtime`, `writ-std` is ordinary Writ code compiled to a normal
module. It imports from `writ-runtime` via standard ModuleRef resolution. `writ-std` is not required for the language to
function — it is a convenience library that can be implemented incrementally.

From the module format's perspective, `writ-runtime` is an ordinary module — its specialness is that the runtime
provides it and the spec mandates its contents.

**Contents of `writ-runtime`:** See §2.18 for the complete manifest of types, contracts, and intrinsic methods that
this module must provide.

## 2.17 Execution Model

The Writ runtime executes IL code as a set of concurrent **tasks**. Each task has its own call stack and executes
independently. The runtime schedules tasks cooperatively — tasks run until they voluntarily suspend at transition
points,
complete, or crash.

The task state machine and scheduling model described below are a **minimum viable reference design** for runtime
implementors. The spec mandates correctness (tasks must execute their IL correctly, defer handlers must fire, atomic
sections must provide exclusion) but does not mandate a specific scheduler, threading model, or state representation.
Runtime implementors may extend the state machine with additional states or transitions as needed for their host
environment.

### 2.17.1 Call Stack

Each task maintains a **managed call stack**: an ordered sequence of **call frames**. Each frame contains:

- **method**: The MethodDef token identifying the executing method.
- **pc**: The program counter — byte offset into the method body's code section.
- **registers**: An array of typed register slots, sized per the method body's `reg_count`.
- **defer_stack**: A LIFO stack of pending defer handler offsets (pushed by `DEFER_PUSH`, popped by `DEFER_POP`).

When a `CALL`, `CALL_VIRT`, `CALL_INDIRECT`, or `CALL_EXTERN` instruction executes, the runtime pushes a new frame.
When `RET` or `RET_VOID` executes, the runtime runs the frame's defer handlers (LIFO), then pops the frame and resumes
the caller. `TAIL_CALL` replaces the current frame rather than pushing a new one.

The call stack is not the native thread stack — it is a runtime-managed data structure. This is essential for
serialization (the runtime must be able to walk and snapshot all frames) and for suspension (the runtime can pause and
resume a task without unwinding native frames).

### 2.17.2 Task States

Each task is in exactly one of the following states:

| State         | Description                                                                                      |
|---------------|--------------------------------------------------------------------------------------------------|
| **Ready**     | Runnable. Waiting to be scheduled for execution.                                                 |
| **Running**   | Actively executing instructions.                                                                 |
| **Suspended** | Blocked. Waiting on a host response, a `JOIN` target, or an atomic lock held by another task.    |
| **Completed** | Finished normally via `RET`/`RET_VOID` from the top frame. Return value is available for `JOIN`. |
| **Cancelled** | Terminated by crash or external `CANCEL`. Defer handlers have run. No return value is produced.  |

Completed and Cancelled are terminal states. `JOIN` on a Completed task delivers the return value. `JOIN` on a
Cancelled task crashes the joining task — there is no return value to deliver.

**Valid transitions:**

| From      | To        | Trigger                                                                      |
|-----------|-----------|------------------------------------------------------------------------------|
| *(new)*   | Ready     | `SPAWN_TASK`, `SPAWN_DETACHED`, or host command (fire event, start dialogue) |
| Ready     | Running   | Scheduler selects the task for execution                                     |
| Running   | Suspended | Task hits a transition point (§2.17.3) or `JOIN` on an incomplete task       |
| Running   | Ready     | Execution limit reached (§2.17.5) — task paused mid-execution                |
| Running   | Completed | `RET`/`RET_VOID` from the top frame (defer handlers run first)               |
| Running   | Cancelled | Crash unwinds all frames (§2.17.7); or `CANCEL` targets this task            |
| Suspended | Ready     | Host confirms the pending request; or `JOIN` target completes/cancels        |
| Ready     | Cancelled | `CANCEL` from another task (defer handlers run)                              |
| Suspended | Cancelled | `CANCEL` from another task (defer handlers run)                              |

**Optional runtime states:** Runtimes may introduce additional states beyond this minimum set. For example, a runtime
implementing atomic sections via a drain-and-run strategy (§2.17.6) may add a **Draining** state: when a task enters
`ATOMIC_BEGIN`, all other Running tasks are moved to Draining (paused at their current instruction boundary), the atomic
section runs to completion, and drained tasks return to Ready. This is one valid approach — not the only one.

### 2.17.3 Transition Points

A **transition point** is an instruction where the runtime suspends the executing task to wait for an external response.
The task moves from Running to Suspended and does not resume until the host or another task provides the awaited result.

| Instruction                                      | Suspension Reason                                                    |
|--------------------------------------------------|----------------------------------------------------------------------|
| `CALL_EXTERN`                                    | Host executes native code. Suspends until host returns a result.     |
| `SET_FIELD` (component field, post-construction) | Proxied to host via suspend-and-confirm (§2.14.2).                   |
| `GET_FIELD` (component field)                    | Host provides the current native value.                              |
| `GET_COMPONENT`                                  | Host resolves whether the entity has the component.                  |
| `SPAWN_ENTITY`                                   | Host creates native representation for attached components.          |
| `INIT_ENTITY`                                    | Flushes buffered component field writes to host as a batch.          |
| `DESTROY_ENTITY`                                 | Notifies host of entity destruction after `on_destroy` completes.    |
| `GET_OR_CREATE`                                  | May trigger entity spawn if the singleton does not yet exist.        |
| `JOIN`                                           | Suspends until the target task reaches Completed or Cancelled state. |

`SET_FIELD` and `GET_FIELD` on **script fields** (not component fields) are direct memory operations and are not
transition points.

Runtime-provided functions (`Runtime.say`, `Runtime.choice`, etc.) suspend through their underlying mechanism — they
are extern calls or intrinsics that internally interact with the host API. They are not special at the IL instruction
level.

**Transition points are the only points where the runtime is guaranteed to be in a consistent, serializable state.**
Save operations (§2.13) should only occur when all running tasks have reached a transition point or are otherwise
suspended.

### 2.17.4 Entry Points

The host drives execution through commands (§2.14.3). The following commands affect the task state machine:

| Host Command       | Effect                                                                                                      |
|--------------------|-------------------------------------------------------------------------------------------------------------|
| **Tick**           | Resume scheduling. The runtime executes Ready tasks until all tasks are Suspended, Completed, or Cancelled. |
| **Fire event**     | Create a new task for the event handler (e.g., `on_interact`). The task enters Ready.                       |
| **Start dialogue** | Create a new task for the dialogue function. The task enters Ready.                                         |
| **Confirm**        | Fulfill a Suspended task's pending host request. The task moves to Ready.                                   |

Tasks created by host commands enter the Ready state. Whether they are scheduled within the current tick or deferred to
the next tick is implementation-defined.

### 2.17.5 Scheduling and Execution Limits

The runtime must schedule Ready tasks for execution. The spec does not mandate a scheduling algorithm — the runtime may
use any strategy (FIFO, priority-based, round-robin, work-stealing, etc.). The order in which Ready tasks are selected
is implementation-defined.

**Threading:** The runtime may execute tasks on a single thread or dispatch them across multiple threads concurrently.
When multiple tasks execute in parallel, the runtime must ensure that `ATOMIC_BEGIN`/`ATOMIC_END` sections provide the
guarantees specified in §2.17.6. The spec recommends multi-threaded dispatch for runtimes targeting modern hardware, but
does not require it.

**Execution limits (recommended):** To prevent runaway scripts from blocking the host engine, runtimes should enforce an
execution limit per tick to bound the total time spent in script execution. This may be:

- An **instruction budget** per task (e.g., 10,000 instructions before yielding).
- A **wall-clock time limit** per tick (e.g., 16ms total across all tasks).
- Any other mechanism appropriate to the host environment.

When a task exceeds the execution limit, the runtime pauses it at the current instruction boundary and moves it from
Running back to Ready. The task resumes from the same instruction on the next scheduling pass. The runtime may log a
warning when a task is repeatedly limit-paused, as this typically indicates a script bug (tight loop with no transition
points).

**Exception:** A task inside an `ATOMIC_BEGIN`/`ATOMIC_END` section must not be paused by execution limits. See §2.17.6.

### 2.17.6 Atomic Sections

`ATOMIC_BEGIN` and `ATOMIC_END` create a region where the runtime **must** guarantee exclusive access to the globals
read or written by the executing task. The following requirements are mandatory:

1. **No interleaving.** While a task is inside an atomic section, no other task may read or write the involved global
   variables until `ATOMIC_END` executes.
2. **No execution-limit suspension.** The runtime must not pause a task due to execution limits while it is inside an
   atomic section. The section runs to `ATOMIC_END` without interruption.
3. **Proper nesting.** Every `ATOMIC_BEGIN` must have a matching `ATOMIC_END` before the enclosing frame returns. The
   runtime may detect unpaired atomics at module load time (verification) or at runtime.

**Implementation guidance:** The spec does not prescribe how the runtime achieves these guarantees. Approaches include
but are not limited to:

- **Drain-and-run:** When a task hits `ATOMIC_BEGIN`, the runtime pauses all other tasks at their next instruction
  boundary (or transition point), runs the atomic section to completion, then resumes normal scheduling. This may use
  an additional task state (e.g., Draining) beyond the minimum set in §2.17.2.
- **Per-global locking:** The atomic section acquires locks on globals as they are accessed. Other tasks block only if
  they attempt to access a locked global.
- **Single-threaded non-preemption:** If all tasks run on a single thread, atomic sections are inherently
  non-interleaved. The runtime simply disables execution-limit pausing for the duration.

**Transition points inside atomic sections:** If a transition point (e.g., `CALL_EXTERN`) occurs inside an atomic
section, the task suspends while holding the atomic guarantee. Other tasks attempting to access the guarded globals will
block until the atomic section completes. This can cause deadlocks if the host response depends on another blocked task.
The compiler **must** emit a warning when a transition point occurs inside an atomic block. A future language mechanism
may allow suppressing this warning for cases where the author has verified safety (see TODO).

### 2.17.7 Crash Propagation and Defer Unwinding

When a task crashes (`CRASH` instruction, failed `UNWRAP`/`UNWRAP_OK`, out-of-bounds array access, dead entity handle
access, division by zero, etc.), the runtime unwinds the **entire task call stack**:

1. Execute all defer handlers on the current frame's defer stack in LIFO order.
2. Pop the frame.
3. Repeat for the next frame down the stack, executing its defer handlers.
4. Continue until all frames are unwound.
5. The task enters the Cancelled state.
6. Log the crash to the host via the runtime logging interface (§2.14.7).

Defer handlers that themselves crash do not halt the unwinding process. The runtime logs the secondary crash and
continues unwinding the remaining defers and frames.

`CANCEL` from another task triggers the same unwinding sequence — the target task's stack is fully unwound with defer
handlers firing at each frame.

### 2.17.8 Task Tree

Tasks form a tree based on their spawn relationships:

- **Scoped tasks** (`SPAWN_TASK`): Children of the spawning task. When the parent completes, crashes, or is cancelled,
  all scoped children are automatically cancelled first (defer handlers run). The parent's own completion is deferred
  until all scoped children have terminated.
- **Detached tasks** (`SPAWN_DETACHED`): Independent of the spawning task. They are not affected by the parent's
  lifecycle and must be explicitly cancelled or allowed to run to completion.

Scoped task cancellation is recursive: cancelling a parent cancels its scoped children, which cancels their scoped
children, and so on. Defer handlers fire at each level during unwinding.

## 2.18 `writ-runtime` Module Contents

The `writ-runtime` module is a virtual module provided by every conforming runtime. It is not compiled from Writ
source — the runtime supplies it as part of its implementation. The compiler references types, contracts, and methods
in `writ-runtime` via standard cross-module TypeRef and MethodRef resolution, exactly as it would reference any other
dependency module.

The spec mandates the types, contracts, and implementations listed below. The runtime must provide these with the
exact layouts specified. Additional types or methods beyond this list are permitted but not required.

### 2.18.1 Core Enums

#### Option\<T\>

```
enum Option<T> {
    None,           // tag 0 — no payload
    Some(value: T), // tag 1 — payload: T
}
```

Tag assignments are mandatory: `None = 0`, `Some = 1`. Zero-initialization of an Option register produces `None`.
The specialized IL instructions (`WRAP_SOME`, `UNWRAP`, `IS_SOME`, `IS_NONE`) depend on these tag values.

The `T?` syntax is sugar for `Option<T>`. The `null` literal is sugar for `Option::None`.

#### Result\<T, E: Error\>

```
enum Result<T, E: Error> {
    Ok(value: T),     // tag 0 — payload: T
    Err(error: E),    // tag 1 — payload: E
}
```

Tag assignments are mandatory: `Ok = 0`, `Err = 1`. The `E` parameter is constrained to the `Error` contract
(§2.18.3). Specialized IL instructions: `WRAP_OK`, `WRAP_ERR`, `UNWRAP_OK`, `IS_OK`, `IS_ERR`, `EXTRACT_ERR`.

### 2.18.2 Range\<T\>

```
struct Range<T> {
    start: T,
    end: T,
    start_inclusive: bool,
    end_inclusive: bool,
}
```

The `..` and `..=` operators construct Range values:

| Syntax  | start_inclusive | end_inclusive |
|---------|-----------------|---------------|
| `a..b`  | `true`          | `false`       |
| `a..=b` | `true`          | `true`        |

Range is a generic struct with no constraints on `T`. The type is deliberately general — while the current syntax
only produces ranges with `start_inclusive = true`, the struct supports all four inclusivity combinations for use by
library code and future language extensions.

**Range iteration:** The runtime provides `Iterable<T>` implementations for `Range<int>` (step by 1) and
`Range<float>` (step by 2.0). User types may provide their own `Iterable<T>` implementations for `Range<UserType>`
to support custom range iteration.

**Range indexing:** Arrays and strings use `Range<int>` for slice operations via `Index<Range<int>, T[]>` and
`Index<Range<int>, string>` (§6.9).

### 2.18.3 Contracts

The following contracts are defined in `writ-runtime`. The compiler maps operator syntax to these contracts
automatically (§10.1, §2.7). Each contract produces a `ContractDef` row in the `writ-runtime` module metadata.

**Arithmetic:**

| Contract    | Method Signature            | Operator  |
|-------------|-----------------------------|-----------|
| `Add<T, R>` | `operator +(other: T) -> R` | `+`       |
| `Sub<T, R>` | `operator -(other: T) -> R` | `-`       |
| `Mul<T, R>` | `operator *(other: T) -> R` | `*`       |
| `Div<T, R>` | `operator /(other: T) -> R` | `/`       |
| `Mod<T, R>` | `operator %(other: T) -> R` | `%`       |
| `Neg<R>`    | `operator -() -> R`         | unary `-` |
| `Not<R>`    | `operator !() -> R`         | `!`       |

**Comparison:**

| Contract | Method Signature                | Operator |
|----------|---------------------------------|----------|
| `Eq<T>`  | `operator ==(other: T) -> bool` | `==`     |
| `Ord<T>` | `operator <(other: T) -> bool`  | `<`      |

Derived operators `!=`, `>`, `<=`, `>=` are compiler desugaring from `Eq` and `Ord` (§17.4).

**Indexing:**

| Contract         | Method Signature                 | Operator   |
|------------------|----------------------------------|------------|
| `Index<K, V>`    | `operator [](key: K) -> V`       | `x[k]`     |
| `IndexSet<K, V>` | `operator []=(key: K, value: V)` | `x[k] = v` |

**Bitwise:**

| Contract       | Method Signature             | Operator |
|----------------|------------------------------|----------|
| `BitAnd<T, R>` | `operator &(other: T) -> R`  | `&`      |
| `BitOr<T, R>`  | `operator \|(other: T) -> R` | `\|`     |

**Iteration:**

| Contract      | Method Signature                   |
|---------------|------------------------------------|
| `Iterable<T>` | `fn iterator(self) -> Iterator<T>` |
| `Iterator<T>` | `fn next(mut self) -> T?`          |

**Conversion and error:**

| Contract  | Method Signature             |
|-----------|------------------------------|
| `Into<T>` | `fn into(self) -> T`         |
| `Error`   | `fn message(self) -> string` |

### 2.18.4 Primitive Pseudo-Types

Primitive types (`int`, `float`, `bool`, `string`) have fixed type tags (§2.15.2) and are not constructed via `NEW`.
To anchor contract implementations in the metadata, `writ-runtime` provides **pseudo-TypeDefs** for each primitive.
These TypeDefs exist solely as targets for `ImplDef` entries — they are not constructable by user code and carry no
user-visible fields or methods beyond their contract implementations.

| Pseudo-TypeDef | Primitive Tag | Purpose                          |
|----------------|---------------|----------------------------------|
| `Int`          | `0x01`        | Anchor for int contract impls    |
| `Float`        | `0x02`        | Anchor for float contract impls  |
| `Bool`         | `0x03`        | Anchor for bool contract impls   |
| `String`       | `0x04`        | Anchor for string contract impls |

The runtime maps primitive type tags to these pseudo-TypeDefs for contract dispatch. When generic code calls a
contract method on a boxed `int`, the runtime resolves via the `Int` pseudo-TypeDef's `ImplDef` entries.

### 2.18.5 Primitive Contract Implementations

All primitive contract implementations are intrinsic — the runtime provides native implementations that correspond
to dedicated IL instructions. For direct primitive operations, the compiler emits the dedicated instruction (§2.7).
The `ImplDef` entries exist for generic dispatch when primitives are boxed through generic parameters.

**int:**

| Contract           | Intrinsic Instruction      |
|--------------------|----------------------------|
| `Add<int, int>`    | `ADD_I`                    |
| `Sub<int, int>`    | `SUB_I`                    |
| `Mul<int, int>`    | `MUL_I`                    |
| `Div<int, int>`    | `DIV_I`                    |
| `Mod<int, int>`    | `MOD_I`                    |
| `Neg<int>`         | `NEG_I`                    |
| `Not<int>`         | `NOT` (bitwise complement) |
| `Eq<int>`          | `CMP_EQ_I`                 |
| `Ord<int>`         | `CMP_LT_I`                 |
| `BitAnd<int, int>` | `BIT_AND`                  |
| `BitOr<int, int>`  | `BIT_OR`                   |
| `Into<float>`      | `I2F`                      |
| `Into<string>`     | `I2S`                      |

**float:**

| Contract            | Intrinsic Instruction |
|---------------------|-----------------------|
| `Add<float, float>` | `ADD_F`               |
| `Sub<float, float>` | `SUB_F`               |
| `Mul<float, float>` | `MUL_F`               |
| `Div<float, float>` | `DIV_F`               |
| `Mod<float, float>` | `MOD_F`               |
| `Neg<float>`        | `NEG_F`               |
| `Eq<float>`         | `CMP_EQ_F`            |
| `Ord<float>`        | `CMP_LT_F`            |
| `Into<int>`         | `F2I`                 |
| `Into<string>`      | `F2S`                 |

**bool:**

| Contract       | Intrinsic Instruction    |
|----------------|--------------------------|
| `Eq<bool>`     | `CMP_EQ_B`               |
| `Not<bool>`    | `NOT` (logical negation) |
| `Into<string>` | `B2S`                    |

**string:**

| Contract                    | Intrinsic                    |
|-----------------------------|------------------------------|
| `Add<string, string>`       | `STR_CONCAT`                 |
| `Eq<string>`                | `CMP_EQ_S`                   |
| `Ord<string>`               | Intrinsic (lexicographic)    |
| `Index<int, string>`        | Intrinsic (single character) |
| `Index<Range<int>, string>` | Intrinsic (substring)        |
| `Into<string>`              | Identity (returns self)      |

### 2.18.6 Array Type

The `Array<T>` TypeDef provides methods and contract implementations for the built-in array type. The `T[]` syntax
is sugar for `Array<T>`. In the type encoding, arrays use kind `0x20` (§2.15.3); the runtime maps this to the
`Array<T>` TypeDef for method resolution and contract dispatch.

**Fields:**

| Field    | Type  | Access    | Intrinsic   |
|----------|-------|-----------|-------------|
| `length` | `int` | Read-only | `ARRAY_LEN` |

**Methods (intrinsic):**

| Method     | Signature                                  | Intrinsic IL   |
|------------|--------------------------------------------|----------------|
| `add`      | `fn add(mut self, item: T)`                | `ARRAY_ADD`    |
| `removeAt` | `fn removeAt(mut self, index: int)`        | `ARRAY_REMOVE` |
| `insert`   | `fn insert(mut self, index: int, item: T)` | `ARRAY_INSERT` |
| `contains` | `fn contains(self, item: T) -> bool`       | Intrinsic      |
| `slice`    | `fn slice(self, range: Range<int>) -> T[]` | `ARRAY_SLICE`  |
| `iterator` | `fn iterator(self) -> Iterator<T>`         | Intrinsic      |

The `contains` method requires `T: Eq` at the call site. This constraint is enforced by the compiler — it is not
encoded on the `Array<T>` TypeDef's generic parameter, which would incorrectly restrict all array usage to
Eq-implementing types.

**Contract implementations:**

| Contract                      | Intrinsic                                         |
|-------------------------------|---------------------------------------------------|
| `Index<int, T>`               | `ARRAY_LOAD` — crashes on out-of-bounds           |
| `IndexSet<int, T>`            | `ARRAY_STORE` — crashes on out-of-bounds          |
| `Index<Range<int>, Array<T>>` | `ARRAY_SLICE`                                     |
| `Iterable<T>`                 | Returns a runtime-provided iterator over elements |

### 2.18.7 Entity Base Type

The `Entity` TypeDef (kind=Entity) serves as the base handle type for all entity references. When a variable is
typed as `Entity` (rather than a specific entity type like `Guard`), it refers to this base type. All user-defined
entity types are assignable to `Entity` for handle operations.

**Static methods (intrinsic):**

| Method        | Signature                            | Intrinsic IL      |
|---------------|--------------------------------------|-------------------|
| `destroy`     | `fn destroy(entity: Entity)`         | `DESTROY_ENTITY`  |
| `isAlive`     | `fn isAlive(entity: Entity) -> bool` | `ENTITY_IS_ALIVE` |
| `getOrCreate` | `fn getOrCreate<T>() -> T`           | `GET_OR_CREATE`   |
| `findAll`     | `fn findAll<T>() -> EntityList<T>`   | `FIND_ALL`        |

### 2.18.8 Versioning

The `writ-runtime` module version tracks the IL specification version. A major version bump in the IL spec
corresponds to a major version bump in `writ-runtime`. Compiled modules reference `writ-runtime` via `ModuleRef`
with a `min_version` matching the IL spec version they were compiled against.

Since `writ-runtime` is provided by the runtime rather than loaded from disk, the runtime ensures its provided module
matches the version expected by loaded user modules. On version mismatch, the runtime reports the conflict via the
logging interface (§2.14.7).

## 3.0 Meta

| Mnemonic | Shape | Operands | Description                                                                                                              |
|----------|-------|----------|--------------------------------------------------------------------------------------------------------------------------|
| `NOP`    | N     | —        | No operation. May be used for alignment or patching.                                                                     |
| `CRASH`  | R     | r_msg    | Crash the current task. r_msg holds a string with the error message. Triggers defer unwinding for the entire task chain. |

## 3.1 Data Movement

| Mnemonic      | Shape | Operands           | Description                                                                                                    |
|---------------|-------|--------------------|----------------------------------------------------------------------------------------------------------------|
| `MOV`         | RR    | r_dst, r_src       | Copy register to register. Semantics depend on type: value copy for primitives, reference copy for heap types. |
| `LOAD_INT`    | RI64  | r_dst, value:i64   | Load 64-bit signed integer literal.                                                                            |
| `LOAD_FLOAT`  | RI64  | r_dst, value:f64   | Load 64-bit IEEE 754 float literal. Same encoding width as LOAD_INT, different interpretation.                 |
| `LOAD_TRUE`   | R     | r_dst              | Load boolean `true`.                                                                                           |
| `LOAD_FALSE`  | R     | r_dst              | Load boolean `false`.                                                                                          |
| `LOAD_STRING` | RI32  | r_dst, str_idx:u32 | Load string reference from the string heap by index.                                                           |
| `LOAD_NULL`   | R     | r_dst              | Load `Option::None` / null.                                                                                    |

## 3.2 Integer Arithmetic

All operands are `int` (64-bit signed). Using these instructions on non-int registers is undefined behavior in the IL (
the compiler must not emit this).

| Mnemonic | Shape | Operands        | Description                          |
|----------|-------|-----------------|--------------------------------------|
| `ADD_I`  | RRR   | r_dst, r_a, r_b | Addition.                            |
| `SUB_I`  | RRR   | r_dst, r_a, r_b | Subtraction.                         |
| `MUL_I`  | RRR   | r_dst, r_a, r_b | Multiplication.                      |
| `DIV_I`  | RRR   | r_dst, r_a, r_b | Division. Crash on division by zero. |
| `MOD_I`  | RRR   | r_dst, r_a, r_b | Modulo. Crash on division by zero.   |
| `NEG_I`  | RR    | r_dst, r_src    | Negation (`-x`).                     |

## 3.3 Float Arithmetic

All operands are `float` (64-bit IEEE 754). IEEE 754 semantics apply throughout (inf, NaN propagation, no crash on /0).

| Mnemonic | Shape | Operands        | Description                                 |
|----------|-------|-----------------|---------------------------------------------|
| `ADD_F`  | RRR   | r_dst, r_a, r_b | Addition.                                   |
| `SUB_F`  | RRR   | r_dst, r_a, r_b | Subtraction.                                |
| `MUL_F`  | RRR   | r_dst, r_a, r_b | Multiplication.                             |
| `DIV_F`  | RRR   | r_dst, r_a, r_b | Division. Returns ±inf or NaN per IEEE 754. |
| `MOD_F`  | RRR   | r_dst, r_a, r_b | Modulo (IEEE 754 remainder).                |
| `NEG_F`  | RR    | r_dst, r_src    | Negation.                                   |

## 3.4 Bitwise & Logical

| Mnemonic  | Shape | Operands        | Description                                 |
|-----------|-------|-----------------|---------------------------------------------|
| `BIT_AND` | RRR   | r_dst, r_a, r_b | Bitwise AND on int.                         |
| `BIT_OR`  | RRR   | r_dst, r_a, r_b | Bitwise OR on int.                          |
| `SHL`     | RRR   | r_dst, r_a, r_b | Shift left. r_b is shift count.             |
| `SHR`     | RRR   | r_dst, r_a, r_b | Arithmetic shift right. r_b is shift count. |
| `NOT`     | RR    | r_dst, r_src    | Logical NOT. Operand must be bool.          |

## 3.5 Comparison

All comparison instructions produce a `bool` result in r_dst.

Primitive comparisons — no dispatch, direct evaluation:

| Mnemonic   | Shape | Operands        | Description                                        |
|------------|-------|-----------------|----------------------------------------------------|
| `CMP_EQ_I` | RRR   | r_dst, r_a, r_b | Integer equality.                                  |
| `CMP_EQ_F` | RRR   | r_dst, r_a, r_b | Float equality. NaN != NaN per IEEE 754.           |
| `CMP_EQ_B` | RRR   | r_dst, r_a, r_b | Bool equality.                                     |
| `CMP_EQ_S` | RRR   | r_dst, r_a, r_b | String equality (value comparison, not reference). |
| `CMP_LT_I` | RRR   | r_dst, r_a, r_b | Integer less-than.                                 |
| `CMP_LT_F` | RRR   | r_dst, r_a, r_b | Float less-than.                                   |

User-type comparisons go through `CALL_VIRT` on `Eq` / `Ord` contracts.

Derived operators (compiler desugars, not in the IL):

- `a != b` → `CMP_EQ` + `NOT`
- `a > b`  → `CMP_LT` with swapped operands (`CMP_LT r, r_b, r_a`)
- `a <= b` → `CMP_LT(a,b) || CMP_EQ(a,b)` or `NOT(CMP_LT(b,a))`
- `a >= b` → `NOT(CMP_LT(a,b))`

## 3.6 Control Flow

| Mnemonic   | Shape | Operands                     | Description                                                                                                                                                                              |
|------------|-------|------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `BR`       | I32   | offset:i32                   | Unconditional relative branch. Offset from the start of this instruction.                                                                                                                |
| `BR_TRUE`  | RI32  | r_cond, offset:i32           | Branch if r_cond is `true`.                                                                                                                                                              |
| `BR_FALSE` | RI32  | r_cond, offset:i32           | Branch if r_cond is `false`.                                                                                                                                                             |
| `SWITCH`   | var   | r_tag, n:u16, offsets:i32[n] | Jump table. r_tag indexes into the offset array. If r_tag < 0 or r_tag >= n, falls through to the next instruction. Encoding: `u16(op) u16(r_tag) u16(n) i32[n]`. Total: `6 + 4n` bytes. |
| `RET`      | R     | r_src                        | Return value from current method. Triggers defer handlers.                                                                                                                               |
| `RET_VOID` | N     | —                            | Return void. Triggers defer handlers.                                                                                                                                                    |

`BR` uses a minimal `I32` shape: `u16(op) + padding:u16 + i32(offset)` = 8 bytes. The padding keeps alignment uniform
with RI32.

## 3.7 Calls

| Mnemonic        | Shape | Operands                                                   | Description                                                                                                                                                                                                                                                                                      |
|-----------------|-------|------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `CALL`          | CALL  | r_dst, method_idx:u32, r_base, argc:u16                    | Static function call. Args in r_base..r_base+argc-1. Return value in r_dst.                                                                                                                                                                                                                      |
| `CALL_VIRT`     | var   | r_dst, r_obj, contract_idx:u32, slot:u16, r_base, argc:u16 | Virtual dispatch. r_obj's runtime type tag resolves the concrete method via `(type_tag, contract_idx, slot)`. r_base..+argc are all args (r_obj should be r_base when self is the first arg). Encoding: `u16(op) u16(r_dst) u16(r_obj) u32(contract_idx) u16(slot) u16(r_base) u16(argc)` = 14B. |
| `CALL_EXTERN`   | CALL  | r_dst, extern_idx:u32, r_base, argc:u16                    | Extern call. extern_idx identifies the declaration in the ExternDef table. Runtime resolves to native code.                                                                                                                                                                                      |
| `NEW_DELEGATE`  | var   | r_dst, method_idx:u32, r_target                            | Create a delegate (function value). r_target is the target object (capture struct, self) or a register holding null for free functions. Method is resolved at creation time. Encoding: `u16(op) u16(r_dst) u32(method_idx) u16(r_target)` = 10B.                                                 |
| `CALL_INDIRECT` | var   | r_dst, r_delegate, r_base, argc:u16                        | Call a delegate. Runtime extracts method + target from the delegate object. If target is non-null, it is prepended as the first argument (self/env). Encoding: `u16(op) u16(r_dst) u16(r_delegate) u16(r_base) u16(argc)` = 10B.                                                                 |
| `TAIL_CALL`     | var   | method_idx:u32, r_base, argc:u16                           | Tail call — reuse the current stack frame. Used for dialogue `->` transitions. No r_dst (does not return to caller). Encoding: `u16(op) u32(method_idx) u16(r_base) u16(argc)` = 10B.                                                                                                            |

## 3.8 Object Model

| Mnemonic          | Shape | Operands                           | Description                                                                                                                                                              |
|-------------------|-------|------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW`             | RI32  | r_dst, type_idx:u32                | Allocate struct instance. Memory is zeroed; defaults and overrides applied via subsequent SET_FIELD instructions.                                                        |
| `GET_FIELD`       | var   | r_dst, r_obj, field_idx:u32        | Read a field from a struct/entity/component. Encoding: `u16(op) u16(r_dst) u16(r_obj) u32(field_idx)` = 10B.                                                             |
| `SET_FIELD`       | var   | r_obj, field_idx:u32, r_val        | Write a field. Encoding: `u16(op) u16(r_obj) u32(field_idx) u16(r_val)` = 10B.                                                                                           |
| `SPAWN_ENTITY`    | RI32  | r_dst, type_idx:u32                | Allocate entity instance, register with entity runtime, notify host to create components with defaults and overrides. Does NOT fire `on_create`.                         |
| `INIT_ENTITY`     | R     | r_entity                           | Fire the entity's `on_create` lifecycle hook. Must be called after field overrides (SET_FIELD) are applied.                                                              |
| `GET_COMPONENT`   | var   | r_dst, r_entity, comp_type_idx:u32 | Access a component on an entity by component type. Returns `Option<Component>`: Some if the entity has the component, None otherwise. Encoding: same as GET_FIELD (10B). |
| `GET_OR_CREATE`   | RI32  | r_dst, singleton_type_idx:u32      | `Entity.getOrCreate<T>()`. Returns the singleton instance, creating it if it doesn't exist.                                                                              |
| `FIND_ALL`        | RI32  | r_dst, entity_type_idx:u32         | `Entity.findAll<T>()`. Returns an EntityList of all live entities of the given type.                                                                                     |
| `DESTROY_ENTITY`  | R     | r_entity                           | Destroy an entity. Fires `on_destroy`, marks entity as dead in the registry, notifies host. Crashes if entity is already dead.                                           |
| `ENTITY_IS_ALIVE` | RR    | r_dst, r_entity                    | Check if entity handle refers to a live entity. Returns bool in r_dst. Does not crash on dead handles.                                                                   |

Construction sequence for `new Guard { name: "Steve" }`:

```
SPAWN_ENTITY  r0, Guard_type      // allocate, register, notify host for components
LOAD_STRING   r1, "Steve"_idx     // load the override value
SET_FIELD     r0, name_field, r1  // override the name field
INIT_ENTITY   r0                  // fire on_create
```

## 3.9 Arrays

| Mnemonic       | Shape | Operands                                | Description                                                                                                                                               |
|----------------|-------|-----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ARRAY`    | RI32  | r_dst, elem_type:u32                    | Create a new empty array with the given element type.                                                                                                     |
| `ARRAY_INIT`   | var   | r_dst, elem_type:u32, count:u16, r_base | Create an array pre-filled from consecutive registers r_base..r_base+count-1. Encoding: `u16(op) u16(r_dst) u32(elem_type) u16(count) u16(r_base)` = 12B. |
| `ARRAY_LOAD`   | RRR   | r_dst, r_arr, r_idx                     | Read element at index. Crash on out-of-bounds.                                                                                                            |
| `ARRAY_STORE`  | RRR   | r_arr, r_idx, r_val                     | Write element at index. Crash on out-of-bounds.                                                                                                           |
| `ARRAY_LEN`    | RR    | r_dst, r_arr                            | Get array length as int.                                                                                                                                  |
| `ARRAY_ADD`    | RR    | r_arr, r_val                            | Append element to end.                                                                                                                                    |
| `ARRAY_REMOVE` | RR    | r_arr, r_idx                            | Remove element at index. Crash on out-of-bounds.                                                                                                          |
| `ARRAY_INSERT` | RRR   | r_arr, r_idx, r_val                     | Insert element at index, shifting subsequent elements.                                                                                                    |
| `ARRAY_SLICE`  | var   | r_dst, r_arr, r_start, r_end            | Create a sub-array copy from index r_start (inclusive) to r_end (exclusive). Encoding: `u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end)` = 10B.     |

## 3.10 Type Operations

**Option (specialized):**

| Mnemonic    | Shape | Operands     | Description                                       |
|-------------|-------|--------------|---------------------------------------------------|
| `WRAP_SOME` | RR    | r_dst, r_val | Construct `Option::Some(val)`.                    |
| `UNWRAP`    | RR    | r_dst, r_opt | Extract value from `Option::Some`. Crash on None. |
| `IS_SOME`   | RR    | r_dst, r_opt | Test if Option is Some -> bool.                   |
| `IS_NONE`   | RR    | r_dst, r_opt | Test if Option is None -> bool.                   |

**Result (specialized):**

| Mnemonic      | Shape | Operands        | Description                                       |
|---------------|-------|-----------------|---------------------------------------------------|
| `WRAP_OK`     | RR    | r_dst, r_val    | Construct `Result::Ok(val)`.                      |
| `WRAP_ERR`    | RR    | r_dst, r_err    | Construct `Result::Err(err)`.                     |
| `UNWRAP_OK`   | RR    | r_dst, r_result | Extract Ok value. Crash on Err.                   |
| `IS_OK`       | RR    | r_dst, r_result | Test if Result is Ok -> bool.                     |
| `IS_ERR`      | RR    | r_dst, r_result | Test if Result is Err -> bool.                    |
| `EXTRACT_ERR` | RR    | r_dst, r_result | Extract the Err value. Undefined if Result is Ok. |

**General enum operations:**

| Mnemonic        | Shape | Operands                                              | Description                                                                                                                                                                                                                                                                              |
|-----------------|-------|-------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ENUM`      | var   | r_dst, type_idx:u32, tag:u16, field_count:u16, r_base | Construct an enum value. tag selects the variant, fields are read from consecutive registers r_base..+field_count. For tag-only variants (no payload), field_count is 0 and r_base is ignored. Encoding: `u16(op) u16(r_dst) u32(type_idx) u16(tag) u16(field_count) u16(r_base)` = 14B. |
| `GET_TAG`       | RR    | r_dst, r_enum                                         | Extract the tag as int.                                                                                                                                                                                                                                                                  |
| `EXTRACT_FIELD` | var   | r_dst, r_enum, field_idx:u16                          | Extract a payload field from the current variant. The caller must have verified the tag first. field_idx is the zero-based index within the variant's payload fields. Encoding: `u16(op) u16(r_dst) u16(r_enum) u16(field_idx)` = 8B.                                                    |

## 3.11 Concurrency

| Mnemonic         | Shape | Operands                                | Description                                                                                                                                                                                                                  |
|------------------|-------|-----------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `SPAWN_TASK`     | CALL  | r_dst, method_idx:u32, r_base, argc:u16 | Spawn a scoped task. Returns a task handle in r_dst. The task is automatically cancelled when the parent scope exits.                                                                                                        |
| `SPAWN_DETACHED` | CALL  | r_dst, method_idx:u32, r_base, argc:u16 | Spawn a detached task. Returns a task handle. The task outlives the parent — it must be explicitly cancelled or run to completion.                                                                                           |
| `JOIN`           | RR    | r_dst, r_handle                         | Suspend until the target task completes. The task's return value is placed in r_dst.                                                                                                                                         |
| `CANCEL`         | R     | r_handle                                | Cancel a task. The target task's defer handlers execute in reverse order before termination.                                                                                                                                 |
| `DEFER_PUSH`     | RI32  | —, handler_offset:i32                   | Push a defer handler onto the current frame's defer stack. handler_offset points to a code block within the current method body. The register slot is unused (padding).                                                      |
| `DEFER_POP`      | N     | —                                       | Pop the topmost defer handler without executing it. Used when execution exits a defer's logical scope without returning from the function — the defer is no longer relevant.                                                 |
| `DEFER_END`      | N     | —                                       | Marks the end of a defer handler block. Signals the runtime to continue the unwind chain (execute the next defer, or complete the return/crash). Only reachable via the defer mechanism — never through normal control flow. |

**Defer layout in the method body:**

Defer handler code lives after the method's main code. It is never reached through normal sequential execution — only
through the defer mechanism on return, crash, or cancellation.

```
    ; --- main code ---
    DEFER_PUSH handler_0         ; register first cleanup
    ...                          ; normal code
    DEFER_PUSH handler_1         ; register second cleanup
    ...                          ; more code
    DEFER_POP                    ; (optional) discard handler_1 if scope exits early
    ...
    RET_VOID                     ; triggers defer stack: handler_1 then handler_0

    ; --- defer handlers (after main code) ---
handler_0:
    CALL _, cleanup_fn, ...
    DEFER_END                    ; continue unwind

handler_1:
    CALL _, other_cleanup, ...
    DEFER_END                    ; continue unwind
```

**When defers execute:**

- On `RET` / `RET_VOID`: all defers on the frame's defer stack execute in reverse order (LIFO), then the return
  completes.
- On crash (`CRASH`, `UNWRAP` failure, out-of-bounds, etc.): defers execute during unwinding.
- On `CANCEL`: the target task's defers execute during cancellation.

**DEFER_POP usage:**
Writ's `defer` runs on function exit, not scope exit. However, `DEFER_POP` is available for the compiler to emit in
cases where a defer becomes logically invalid — for example, if a resource is manually cleaned up before the function
returns, the compiler can pop the defer that would have cleaned it up. This is an optimization, not a semantic
requirement. If `DEFER_POP` is never emitted, all defers simply fire on return (which is correct per the spec).

## 3.12 Globals & Atomics

| Mnemonic       | Shape | Operands              | Description                                                                                                               |
|----------------|-------|-----------------------|---------------------------------------------------------------------------------------------------------------------------|
| `LOAD_GLOBAL`  | RI32  | r_dst, global_idx:u32 | Read a global variable. The runtime ensures atomic read semantics.                                                        |
| `STORE_GLOBAL` | var   | global_idx:u32, r_src | Write a global variable. The runtime ensures atomic write semantics. Encoding: `u16(op) u32(global_idx) u16(r_src)` = 8B. |
| `ATOMIC_BEGIN` | N     | —                     | Enter an atomic section. The runtime guarantees no other task reads or writes the involved globals until ATOMIC_END.      |
| `ATOMIC_END`   | N     | —                     | Exit the atomic section.                                                                                                  |

`ATOMIC_BEGIN` / `ATOMIC_END` must be properly nested. An ATOMIC_BEGIN without a matching ATOMIC_END before function
exit is a verification error. The runtime MAY detect this at load time or at runtime.

## 3.13 Conversion

**Primitive-to-primitive (specialized, no dispatch):**

| Mnemonic | Shape | Operands     | Description                                                 |
|----------|-------|--------------|-------------------------------------------------------------|
| `I2F`    | RR    | r_dst, r_src | int -> float. Exact when possible, nearest float otherwise. |
| `F2I`    | RR    | r_dst, r_src | float -> int. Truncation toward zero.                       |
| `I2S`    | RR    | r_dst, r_src | int -> string. Decimal representation.                      |
| `F2S`    | RR    | r_dst, r_src | float -> string. Runtime-defined precision.                 |
| `B2S`    | RR    | r_dst, r_src | bool -> string. `"true"` or `"false"`.                      |

**General conversion (user types):**

| Mnemonic  | Shape | Operands                      | Description                                                                                                                                                                |
|-----------|-------|-------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `CONVERT` | var   | r_dst, r_src, target_type:u32 | Invoke `Into<T>` for the value in r_src, where T is target_type. Dispatches through the contract system. Encoding: `u16(op) u16(r_dst) u16(r_src) u32(target_type)` = 10B. |

## 3.14 Strings

| Mnemonic     | Shape | Operands                 | Description                                                                                                                                                                                                 |
|--------------|-------|--------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `STR_CONCAT` | RRR   | r_dst, r_a, r_b          | Concatenate two strings. Returns a new string.                                                                                                                                                              |
| `STR_BUILD`  | var   | r_dst, count:u16, r_base | Concatenate count consecutive string registers r_base..+count into one string. Optimized for formattable string lowering (`$"HP: {hp}/{max}"`). Encoding: `u16(op) u16(r_dst) u16(count) u16(r_base)` = 8B. |
| `STR_LEN`    | RR    | r_dst, r_str             | String length in characters (not bytes).                                                                                                                                                                    |

## 3.15 Boxing

Boxing is required when value types (`int`, `float`, `bool`, enums) pass through generic parameters. The compiler emits
`BOX` before passing a value type to a generic parameter and `UNBOX` when extracting a concrete value type from a
generic return or field.

| Mnemonic | Shape | Operands     | Description                                                                                                                                                                                                       |
|----------|-------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `BOX`    | RR    | r_dst, r_src | Heap-allocate a boxed object containing the value in r_src. r_dst receives a reference to the box. The runtime reads r_src's declared type from the register type table to determine the box layout and type tag. |
| `UNBOX`  | RR    | r_dst, r_src | Extract the value from a boxed reference in r_src into r_dst. The runtime reads r_dst's declared type to verify the box contents match. Crash on type mismatch.                                                   |

No explicit type token is needed — registers are abstract typed slots (§2.5), so the runtime already knows every
register's type from the method body's register type table (§2.16.6).

**When the compiler emits boxing:**

```
// Source: fn identity<T>(val: T) -> T { val }
// Call site: let x = identity(42);

// Caller:
LOAD_INT    r0, 42
BOX         r1, r0              // box int for generic param
CALL        r2, identity, r1, 1
UNBOX       r3, r2              // unbox return value back to int
```

Reference types (`string`, structs, arrays, entities, delegates) are already references and pass through generics
without boxing.

## 3.16 Serialization Control — REMOVED

The `CRITICAL_BEGIN` / `CRITICAL_END` instructions have been removed. The suspend-and-confirm model (§2.14.2 in design
decisions) ensures serialization only occurs at transition points, making explicit critical sections unnecessary.

## 4.0 Instruction Count by Category

| Category           | Count  | Instruction Mnemonics                                                                                                         |
|--------------------|--------|-------------------------------------------------------------------------------------------------------------------------------|
| Meta               | 2      | NOP, CRASH                                                                                                                    |
| Data Movement      | 7      | MOV, LOAD_INT, LOAD_FLOAT, LOAD_TRUE, LOAD_FALSE, LOAD_STRING, LOAD_NULL                                                      |
| Integer Arithmetic | 6      | ADD_I, SUB_I, MUL_I, DIV_I, MOD_I, NEG_I                                                                                      |
| Float Arithmetic   | 6      | ADD_F, SUB_F, MUL_F, DIV_F, MOD_F, NEG_F                                                                                      |
| Bitwise & Logical  | 5      | BIT_AND, BIT_OR, SHL, SHR, NOT                                                                                                |
| Comparison         | 6      | CMP_EQ_I, CMP_EQ_F, CMP_EQ_B, CMP_EQ_S, CMP_LT_I, CMP_LT_F                                                                    |
| Control Flow       | 6      | BR, BR_TRUE, BR_FALSE, SWITCH, RET, RET_VOID                                                                                  |
| Calls & Delegates  | 6      | CALL, CALL_VIRT, CALL_EXTERN, NEW_DELEGATE, CALL_INDIRECT, TAIL_CALL                                                          |
| Object Model       | 10     | NEW, GET_FIELD, SET_FIELD, SPAWN_ENTITY, INIT_ENTITY, GET_COMPONENT, GET_OR_CREATE, FIND_ALL, DESTROY_ENTITY, ENTITY_IS_ALIVE |
| Arrays             | 9      | NEW_ARRAY, ARRAY_INIT, ARRAY_LOAD, ARRAY_STORE, ARRAY_LEN, ARRAY_ADD, ARRAY_REMOVE, ARRAY_INSERT, ARRAY_SLICE                 |
| Option             | 4      | WRAP_SOME, UNWRAP, IS_SOME, IS_NONE                                                                                           |
| Result             | 6      | WRAP_OK, WRAP_ERR, UNWRAP_OK, IS_OK, IS_ERR, EXTRACT_ERR                                                                      |
| Enum               | 3      | NEW_ENUM, GET_TAG, EXTRACT_FIELD                                                                                              |
| Concurrency        | 7      | SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH, DEFER_POP, DEFER_END                                                    |
| Globals & Atomics  | 4      | LOAD_GLOBAL, STORE_GLOBAL, ATOMIC_BEGIN, ATOMIC_END                                                                           |
| Conversion         | 6      | I2F, F2I, I2S, F2S, B2S, CONVERT                                                                                              |
| Strings            | 3      | STR_CONCAT, STR_BUILD, STR_LEN                                                                                                |
| Boxing             | 2      | BOX, UNBOX                                                                                                                    |
| **Total**          | **91** |                                                                                                                               |

## 4.1 Instruction Shape Reference

| Shape  | Layout           | Size   | Byte Breakdown                                      |
|--------|------------------|--------|-----------------------------------------------------|
| `N`    | `op`             | 2B     | `u16(op)`                                           |
| `R`    | `op r`           | 4B     | `u16(op) u16(r)`                                    |
| `RR`   | `op r r`         | 6B     | `u16(op) u16(r) u16(r)`                             |
| `RRR`  | `op r r r`       | 8B     | `u16(op) u16(r) u16(r) u16(r)`                      |
| `RI32` | `op r i32`       | 8B     | `u16(op) u16(r) u32(imm)`                           |
| `RI64` | `op r i64`       | 12B    | `u16(op) u16(r) u64(imm)`                           |
| `I32`  | `op pad i32`     | 8B     | `u16(op) u16(pad) i32(imm)` — used by BR            |
| `CALL` | `op r i32 r u16` | 12B    | `u16(op) u16(r_dst) u32(idx) u16(r_base) u16(argc)` |
| `var`  | per-instruction  | varies | Documented per instruction in §2                    |

## 4.2 Opcode Assignment Table

Opcodes are partitioned by category in the high byte (see §2.5 for the scheme).

### 0x00 — Meta

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0000` | NOP      | N     |
| `0x0001` | CRASH    | R     |

### 0x01 — Data Movement

| Opcode   | Mnemonic    | Shape |
|----------|-------------|-------|
| `0x0100` | MOV         | RR    |
| `0x0101` | LOAD_INT    | RI64  |
| `0x0102` | LOAD_FLOAT  | RI64  |
| `0x0103` | LOAD_TRUE   | R     |
| `0x0104` | LOAD_FALSE  | R     |
| `0x0105` | LOAD_STRING | RI32  |
| `0x0106` | LOAD_NULL   | R     |

### 0x02 — Integer Arithmetic

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0200` | ADD_I    | RRR   |
| `0x0201` | SUB_I    | RRR   |
| `0x0202` | MUL_I    | RRR   |
| `0x0203` | DIV_I    | RRR   |
| `0x0204` | MOD_I    | RRR   |
| `0x0205` | NEG_I    | RR    |

### 0x03 — Float Arithmetic

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0300` | ADD_F    | RRR   |
| `0x0301` | SUB_F    | RRR   |
| `0x0302` | MUL_F    | RRR   |
| `0x0303` | DIV_F    | RRR   |
| `0x0304` | MOD_F    | RRR   |
| `0x0305` | NEG_F    | RR    |

### 0x04 — Bitwise & Logical

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0400` | BIT_AND  | RRR   |
| `0x0401` | BIT_OR   | RRR   |
| `0x0402` | SHL      | RRR   |
| `0x0403` | SHR      | RRR   |
| `0x0404` | NOT      | RR    |

### 0x05 — Comparison

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0500` | CMP_EQ_I | RRR   |
| `0x0501` | CMP_EQ_F | RRR   |
| `0x0502` | CMP_EQ_B | RRR   |
| `0x0503` | CMP_EQ_S | RRR   |
| `0x0504` | CMP_LT_I | RRR   |
| `0x0505` | CMP_LT_F | RRR   |

### 0x06 — Control Flow

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0600` | BR       | I32   |
| `0x0601` | BR_TRUE  | RI32  |
| `0x0602` | BR_FALSE | RI32  |
| `0x0603` | SWITCH   | var   |
| `0x0604` | RET      | R     |
| `0x0605` | RET_VOID | N     |

### 0x07 — Calls & Delegates

| Opcode   | Mnemonic      | Shape |
|----------|---------------|-------|
| `0x0700` | CALL          | CALL  |
| `0x0701` | CALL_VIRT     | var   |
| `0x0702` | CALL_EXTERN   | CALL  |
| `0x0703` | NEW_DELEGATE  | var   |
| `0x0704` | CALL_INDIRECT | var   |
| `0x0705` | TAIL_CALL     | var   |

### 0x08 — Object Model

| Opcode   | Mnemonic        | Shape |
|----------|-----------------|-------|
| `0x0800` | NEW             | RI32  |
| `0x0801` | GET_FIELD       | var   |
| `0x0802` | SET_FIELD       | var   |
| `0x0803` | SPAWN_ENTITY    | RI32  |
| `0x0804` | INIT_ENTITY     | R     |
| `0x0805` | GET_COMPONENT   | var   |
| `0x0806` | GET_OR_CREATE   | RI32  |
| `0x0807` | FIND_ALL        | RI32  |
| `0x0808` | DESTROY_ENTITY  | R     |
| `0x0809` | ENTITY_IS_ALIVE | RR    |

### 0x09 — Arrays

| Opcode   | Mnemonic     | Shape |
|----------|--------------|-------|
| `0x0900` | NEW_ARRAY    | RI32  |
| `0x0901` | ARRAY_INIT   | var   |
| `0x0902` | ARRAY_LOAD   | RRR   |
| `0x0903` | ARRAY_STORE  | RRR   |
| `0x0904` | ARRAY_LEN    | RR    |
| `0x0905` | ARRAY_ADD    | RR    |
| `0x0906` | ARRAY_REMOVE | RR    |
| `0x0907` | ARRAY_INSERT | RRR   |
| `0x0908` | ARRAY_SLICE  | var   |

### 0x0A — Type Operations

**Option (0x0A00–0x0A0F):**

| Opcode   | Mnemonic  | Shape |
|----------|-----------|-------|
| `0x0A00` | WRAP_SOME | RR    |
| `0x0A01` | UNWRAP    | RR    |
| `0x0A02` | IS_SOME   | RR    |
| `0x0A03` | IS_NONE   | RR    |

**Result (0x0A10–0x0A1F):**

| Opcode   | Mnemonic    | Shape |
|----------|-------------|-------|
| `0x0A10` | WRAP_OK     | RR    |
| `0x0A11` | WRAP_ERR    | RR    |
| `0x0A12` | UNWRAP_OK   | RR    |
| `0x0A13` | IS_OK       | RR    |
| `0x0A14` | IS_ERR      | RR    |
| `0x0A15` | EXTRACT_ERR | RR    |

**Enum (0x0A20–0x0A2F):**

| Opcode   | Mnemonic      | Shape |
|----------|---------------|-------|
| `0x0A20` | NEW_ENUM      | var   |
| `0x0A21` | GET_TAG       | RR    |
| `0x0A22` | EXTRACT_FIELD | var   |

### 0x0B — Concurrency

| Opcode   | Mnemonic       | Shape |
|----------|----------------|-------|
| `0x0B00` | SPAWN_TASK     | CALL  |
| `0x0B01` | SPAWN_DETACHED | CALL  |
| `0x0B02` | JOIN           | RR    |
| `0x0B03` | CANCEL         | R     |
| `0x0B04` | DEFER_PUSH     | RI32  |
| `0x0B05` | DEFER_POP      | N     |
| `0x0B06` | DEFER_END      | N     |

### 0x0C — Globals & Atomics

| Opcode   | Mnemonic     | Shape |
|----------|--------------|-------|
| `0x0C00` | LOAD_GLOBAL  | RI32  |
| `0x0C01` | STORE_GLOBAL | var   |
| `0x0C02` | ATOMIC_BEGIN | N     |
| `0x0C03` | ATOMIC_END   | N     |

### 0x0D — Conversion

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0D00` | I2F      | RR    |
| `0x0D01` | F2I      | RR    |
| `0x0D02` | I2S      | RR    |
| `0x0D03` | F2S      | RR    |
| `0x0D04` | B2S      | RR    |
| `0x0D05` | CONVERT  | var   |

### 0x0E — Strings

| Opcode   | Mnemonic   | Shape |
|----------|------------|-------|
| `0x0E00` | STR_CONCAT | RRR   |
| `0x0E01` | STR_BUILD  | var   |
| `0x0E02` | STR_LEN    | RR    |

### 0x0F — Boxing

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0F00` | BOX      | RR    |
| `0x0F01` | UNBOX    | RR    |

# Appendix

Supplementary material including open design questions and a consolidated log of IL design decisions.

---

## A. Open Questions

The following design questions remain unresolved and are tracked for future specification revisions.

**String formatting**
Should there be a format specifier syntax within interpolation? E.g., `{price:.2f}` for float formatting.

**Tuple types**
Are anonymous tuple types `(int, string)` useful, or are named structs sufficient?

**Destructuring**
Should `let (x, y) = getPosition();` or `let { name, gold } = merchant;` be supported?

**Standard library API**
Exact API surface for `List<T>`, `Map<K,V>`, `Set<T>`, `EntityList<T>`, iterators, string utilities, math functions.
Core array operations (`T[]`) are defined in Section 6.3. Relationship between arrays and `List<T>` (wrapper, alias, or
distinct type) is TBD.

**REPL / hot reload**
Should the runtime support live script reloading during development?

**Entity destruction semantics**
What does `Entity.getOrCreate` return for a destroyed singleton? Recreate silently, return Option, or crash?

**Component dynamic add/remove**
Components are extern and data-only (host-provided). The component set is fixed at construction time by the entity
declaration. Dynamic add/remove would require host engine support and is not a language-level feature.

**EntityList.with\<T\>() narrowing**
Should `.with<Component>()` refine the type so that component access is guaranteed non-optional within the filtered set?

**Custom attributes**
Can users define their own attributes, or are they limited to builtin ones?

**Serialization**
How are entity and game state serialized for save/load? Automatic, opt-in via attribute, or manual?

**Localization pluralization**
How should pluralization rules be handled across locales (e.g., `{count} items` where some languages have complex plural
forms)?

**Properties & UI binding**
Should Writ have a `property` keyword with get/set accessors, an `[Observable]` attribute, or both? How should game UI
bind to script-side state — push-based (change notifications), pull-based (polling), or a declarative binding syntax?

## B. IL Decision Log

| Decision                     | Choice                                             | Rationale                                                                                                         |
|------------------------------|----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| Stack vs Register VM         | **Register-based**                                 | Explicit state aids serialization; virtual registers avoid alloc complexity                                       |
| Type preservation            | **Typed IL (CLR-style)**                           | Generics preserved, reflection support, JIT specialization possible                                               |
| Execution model              | **Cooperative yielding, preemptive serialization** | Functions are normal code; runtime manages suspension at transition points                                        |
| Binary format                | **Binary from day one**                            | Text format can be added later; binary is the primary artifact                                                    |
| Opcode width                 | **u16**                                            | Future-proof against running out of opcode space                                                                  |
| Register addressing          | **u16**                                            | Up to 65535 registers per function                                                                                |
| Table indices                | **u32**                                            | Up to ~4 billion entries per table                                                                                |
| Enum no-payload variants     | **Tag-only, no payload space**                     | Saves space, tag is sufficient                                                                                    |
| Operator overloading in IL   | **Compiler concern**                               | Primitives use typed instructions; user-type ops dispatch through CALL_VIRT                                       |
| BR padding                   | **Pad to 8B**                                      | Parse simplicity over 2-byte savings                                                                              |
| Bulk string concat           | **STR_BUILD**                                      | Common operation in game scripting (formattable strings)                                                          |
| Option/Result specialization | **Both specialized + general enum**                | Common types get fast path, general instructions for user types                                                   |
| DEFER_POP                    | **Keep it**                                        | Optimization for compiler to discard irrelevant defers early                                                      |
| Memory model                 | **GC-assumed**                                     | Language semantics designed for GC; runtime implementors choose algorithm                                         |
| Structs                      | **Reference types**                                | Heap-allocated, GC-managed. Assignment copies reference (shared object).                                          |
| Enums                        | **Value types**                                    | Tag + inline payload. Copied on assignment. Reference payloads are GC-traced.                                     |
| Binding mutability           | **Binding-only**                                   | `let`/`let mut` controls the binding, not the object. Standard GC-language model.                                 |
| Closure/function values      | **Delegate model (C# style)**                      | Unified: closures, function refs, and bound methods are all delegates                                             |
| Closure mut captures         | **Shared capture struct**                          | Compiler generates a struct; both outer scope and closure reference it                                            |
| Empty closures               | **Null target optimization**                       | No capture struct allocated if nothing is captured                                                                |
| Dead entity access           | **Crash**                                          | Accessing a destroyed entity handle crashes the task                                                              |
| Entity ownership             | **Runtime owns script state, host owns native**    | Extern component fields proxied through host API                                                                  |
| Save/load IL                 | **Include original IL in save**                    | PCs become invalid if scripts are recompiled                                                                      |
| Self parameter               | **Explicit `self`/`mut self`**                     | Methods take explicit receiver; operators and lifecycle hooks have implicit self                                  |
| Binding mutability           | **Strict (prevents mutation)**                     | `let` prevents both reassignment and mutation through the binding                                                 |
| Component back-ref           | **Hidden `@entity` field**                         | Compiler-emitted, unreachable from script; set during SPAWN_ENTITY, used internally for component access lowering |
| Construction syntax          | **`new Type { ... }`**                             | `new` keyword disambiguates construction from blocks; same syntax for structs and entities                        |
| Default field values         | **Runtime expressions, inlined**                   | Compiler emits default expression code at each construction site; `NEW` allocates zeroed                          |
| Components                   | **Extern-only, data-only**                         | No script-defined components; components are host-provided data schemas, no methods                               |
| Lifecycle hooks              | **Universal `on` hooks**                           | `on create/finalize/serialize/deserialize` on structs and entities; `on destroy/interact` entity-only             |
| Host communication           | **Suspend-and-confirm**                            | Runtime suspends on host operations until host confirms; aligns with game engine logic loop                       |
| Registers                    | **Abstract typed slots**                           | Each register holds one value of declared type; runtime determines physical storage                               |
| Primitive type tags          | **Fixed u8 tags (0x00–0x04)**                      | void, int, float, bool, string — self-describing, no payload                                                      |
| TypeRef encoding             | **Variable-length blob**                           | Kind byte + payload; covers primitives, TypeDef, TypeSpec, GenericParam, Array, function types                    |
| TypeDef table                | **Single table**                                   | Structs, enums, entities, components share one table; `kind` flag distinguishes                                   |
| Option/Result types          | **Regular generic enums**                          | No special type encoding; specialness at instruction level only                                                   |
| Generic dispatch             | **Boxing (CLR model)**                             | Value types boxed when passed through generic params; dispatch via contract table                                 |
| Enum tag                     | **u16 discriminant**                               | Up to 65535 variants; total size = tag + largest payload, all variants padded                                     |
| Option null-ptr opt          | **Permitted, not mandated**                        | Runtime may optimize Option<ref> to bare nullable pointer                                                         |
| Module format                | **Binary, 200-byte fixed header**                  | Magic `WRIT`, format version u16, heaps + 21-table directory                                                      |
| Multi-module linking         | **Name-based resolution at load time**             | DAG of modules; TypeRef/MethodRef/FieldRef resolved by name                                                       |
| Metadata tokens              | **u32: top 8 = table ID, bottom 24 = row**         | Uniform encoding for all cross-table references                                                                   |
| Module versioning            | **Semver 3.0.0 (MAJOR.MINOR.PATCH)**               | ModuleRef carries min_version; same-major compatibility rule                                                      |
| String/blob heaps            | **Length-prefixed**                                | u32(length) + bytes; offset 0 = empty/null                                                                        |
| Entity construction          | **INIT_ENTITY commits buffered writes**            | Component writes buffered during construction, flushed as batch                                                   |
| Well-known type table        | **Removed**                                        | Core types in `writ-runtime` module; referenced via standard TypeRef                                              |
| `writ-runtime` module        | **Runtime-provided, spec-mandated**                | Intrinsic flag on MethodDefs; runtime implements natively                                                         |
| `writ-std` module            | **Optional, written in Writ**                      | Utility types (List, Map, etc.); imports from writ-runtime                                                        |
| Circular module deps         | **Forbidden (DAG)**                                | Load order follows dependency graph                                                                               |
| Cross-module fields          | **FieldRef (name-based, ABI-safe)**                | Field reordering in dependency doesn't break dependents                                                           |
| Boxing instructions          | **BOX/UNBOX (RR shape)**                           | Runtime reads register type table — no redundant type token needed                                                |
| TYPE_CHECK instruction       | **Dropped**                                        | No `is` operator, no inheritance, no `any` type — all cases covered by GET_TAG, IS_SOME, CALL_VIRT                |
| LOAD_CONST instruction       | **Folded into LOAD_GLOBAL**                        | GlobalDef covers both; runtime optimizes constant reads via mutability flag                                       |
| Opcode numbering             | **High byte = category, low byte = instruction**   | 16 categories × 256 slots; sub-ranges within 0x0A for Option/Result/Enum                                          |
| Entity handles               | **Opaque registry-based handles**                  | Not direct GC pointers; runtime resolves against entity registry; dead handles crash on access                    |
| Entity destroy/isAlive       | **Static methods in Entity namespace**             | `Entity.destroy(e)` / `Entity.isAlive(e)` — not instance methods; lower to DESTROY_ENTITY / ENTITY_IS_ALIVE       |
| Tick execution               | **Run-to-suspension, cooperative**                 | Tasks run until suspend/complete/crash; execution limits recommended, not mandated                                |
| Task states                  | **Ready/Running/Suspended/Completed/Cancelled**    | Minimum viable set; runtimes may extend (e.g., Draining for atomics)                                              |
| Threading                    | **Recommended, not required**                      | Multi-thread dispatch supported; atomic exclusion must be guaranteed                                              |
| Atomic sections              | **Hard guarantees, implementation free**           | No interleaving, no budget pause; drain-and-run / locking / single-thread all valid                               |
| Transition in atomic         | **Compiler MUST warn**                             | Deadlock risk; future suppression mechanism (TODO A8)                                                             |
| Crash unwinding              | **Full stack, all defers**                         | Every frame unwound; secondary defer crashes logged and skipped                                                   |
| JOIN cancelled task          | **Crash**                                          | No return value exists; joining task crashes                                                                      |
