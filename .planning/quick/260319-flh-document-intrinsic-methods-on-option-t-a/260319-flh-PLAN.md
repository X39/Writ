---
phase: quick
plan: 260319-flh
type: execute
wave: 1
depends_on: []
files_modified:
  - language-spec/spec/47_2_18_writ_runtime_module_contents.md
autonomous: true
requirements: []
must_haves:
  truths:
    - "Option<T> intrinsic methods (is_some, is_none, unwrap) are documented with signatures and IL mapping"
    - "Result<T, E> intrinsic methods (is_ok, is_err, unwrap, unwrap_err) are documented with signatures and IL mapping"
    - "The relationship between unwrap methods and the ! postfix operator is explicitly documented"
    - "Method tables follow the exact format used in section 2.18.6 Array<T>"
  artifacts:
    - path: "language-spec/spec/47_2_18_writ_runtime_module_contents.md"
      provides: "Updated spec with Option and Result intrinsic method tables"
      contains: "Methods (intrinsic)"
  key_links: []
---

<objective>
Document intrinsic methods on Option<T> and Result<T, E> in IL spec section 2.18.1.

Purpose: The compiler and LSP already implement is_some, is_none, unwrap, is_ok, is_err, unwrap_err as intrinsic methods on these types, but the spec (the source of truth) does not document them as callable methods. This closes the gap between implementation and specification.

Output: Updated `language-spec/spec/47_2_18_writ_runtime_module_contents.md` with intrinsic method tables for both core enum types.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@language-spec/spec/47_2_18_writ_runtime_module_contents.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add intrinsic method tables to Option and Result in section 2.18.1</name>
  <files>language-spec/spec/47_2_18_writ_runtime_module_contents.md</files>
  <action>
Edit the file `language-spec/spec/47_2_18_writ_runtime_module_contents.md` to add intrinsic method documentation to the two core enums in section 2.18.1. Make the following changes:

**1. Option section (after the `T?` / `null` sugar paragraph, around line 27):**

Add a **Methods (intrinsic):** table matching the exact format of the Array methods table in section 2.18.6:

```
**Methods (intrinsic):**

| Method    | Signature                  | Intrinsic IL |
|-----------|----------------------------|--------------|
| `is_some` | `fn is_some(self) -> bool` | `IS_SOME`    |
| `is_none` | `fn is_none(self) -> bool` | `IS_NONE`    |
| `unwrap`  | `fn unwrap(self) -> T`     | `UNWRAP`     |
```

After the table, add a paragraph:

```
The `unwrap` method crashes the task if the value is `None`. This is equivalent to the `!` postfix operator — `opt.unwrap()` and `opt!` produce identical IL.
```

**2. Update the existing Option "specialized IL instructions" sentence (line 24):**

Change the sentence:
```
The specialized IL instructions (`WRAP_SOME`, `UNWRAP`, `IS_SOME`, `IS_NONE`) depend on these tag values.
```
To:
```
The specialized IL instructions (`WRAP_SOME`, `UNWRAP`, `IS_SOME`, `IS_NONE`) depend on these tag values. The query and extraction instructions are exposed as intrinsic methods — see table below.
```

**3. Result section (after the "Specialized IL instructions" sentence, around line 38):**

Add a **Methods (intrinsic):** table:

```
**Methods (intrinsic):**

| Method       | Signature                  | Intrinsic IL  |
|--------------|----------------------------|---------------|
| `is_ok`      | `fn is_ok(self) -> bool`   | `IS_OK`       |
| `is_err`     | `fn is_err(self) -> bool`  | `IS_ERR`      |
| `unwrap`     | `fn unwrap(self) -> T`     | `UNWRAP_OK`   |
| `unwrap_err` | `fn unwrap_err(self) -> E` | `EXTRACT_ERR` |
```

After the table, add a paragraph:

```
The `unwrap` method crashes the task if the value is `Err`. This is equivalent to the `!` postfix operator — `result.unwrap()` and `result!` produce identical IL.
```

**4. Update the existing Result "Specialized IL instructions" sentence (line 38):**

Change the sentence:
```
Specialized IL instructions: `WRAP_OK`, `WRAP_ERR`, `UNWRAP_OK`, `IS_OK`, `IS_ERR`, `EXTRACT_ERR`.
```
To:
```
Specialized IL instructions: `WRAP_OK`, `WRAP_ERR`, `UNWRAP_OK`, `IS_OK`, `IS_ERR`, `EXTRACT_ERR`. The query and extraction instructions are exposed as intrinsic methods — see table below.
```

**Formatting rules:**
- Use backtick-quoted method names, signatures, and IL instruction names in the table (matching section 2.18.6 style exactly)
- Use `**Methods (intrinsic):**` as the bold heading (matching section 2.18.6 exactly)
- Do NOT modify any other sections (2.18.2 through 2.18.8 must remain unchanged)
- Blank line before and after the table and the explanatory paragraph
  </action>
  <verify>
    <automated>node -e "const fs=require('fs'); const c=fs.readFileSync('language-spec/spec/47_2_18_writ_runtime_module_contents.md','utf8'); const checks=['Methods (intrinsic)','is_some','is_none','unwrap','is_ok','is_err','unwrap_err','EXTRACT_ERR','postfix operator','see table below']; const missing=checks.filter(s=>!c.includes(s)); if(missing.length){console.error('MISSING:',missing);process.exit(1)}else{console.log('All expected content present');const count=(c.match(/Methods \(intrinsic\)/g)||[]).length;if(count<2){console.error('Expected 2+ Methods (intrinsic) headings, found',count);process.exit(1)}console.log('Found',count,'Methods (intrinsic) sections')}"</automated>
  </verify>
  <done>
    - Option section has a Methods (intrinsic) table with is_some, is_none, unwrap and their IL mappings
    - Result section has a Methods (intrinsic) table with is_ok, is_err, unwrap, unwrap_err and their IL mappings
    - Both sections document the unwrap/! operator equivalence
    - Both existing "specialized IL instructions" sentences now reference the method tables
    - No other sections of the file are modified
    - Table formatting matches section 2.18.6 Array methods style exactly
  </done>
</task>

</tasks>

<verification>
- File contains exactly 3 "Methods (intrinsic)" headings (Option, Result, and existing Array)
- Option table has 3 rows (is_some, is_none, unwrap)
- Result table has 4 rows (is_ok, is_err, unwrap, unwrap_err)
- Both unwrap paragraphs mention the `!` postfix operator equivalence
- Sections 2.18.2 through 2.18.8 are unchanged from the original
</verification>

<success_criteria>
The IL spec section 2.18.1 documents all intrinsic methods on Option<T> and Result<T, E> with their signatures and IL instruction mappings, following the same formatting conventions as the Array<T> methods table in section 2.18.6.
</success_criteria>

<output>
After completion, create `.planning/quick/260319-flh-document-intrinsic-methods-on-option-t-a/260319-flh-SUMMARY.md`
</output>
