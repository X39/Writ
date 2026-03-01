---
phase: quick
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - README.md
  - CONTRIBUTORS.md
autonomous: true
requirements:
  - QUICK-README
must_haves:
  truths:
    - "README.md describes the Writ project accurately with current status"
    - "README.md contains the Contributing section with Code of Conduct and CLA verbatim"
    - "CONTRIBUTORS.md exists and is referenced from README"
  artifacts:
    - path: "README.md"
      provides: "Project overview, features, structure, status, license, contributing with CLA"
      contains: "Contributor License Agreement"
    - path: "CONTRIBUTORS.md"
      provides: "Contributors list file"
      contains: "Contributors"
  key_links:
    - from: "README.md"
      to: "CONTRIBUTORS.md"
      via: "markdown link in CLA section"
      pattern: "\\[CONTRIBUTORS\\]\\(CONTRIBUTORS\\.md\\)"
---

<objective>
Recreate README.md with comprehensive project information and add GitHub community files.

Purpose: The current README.md is outdated (references a Google Doc spec link, says "License TBD", lacks contributing guidelines). Replace it with an accurate, complete README reflecting the current state of the Writ toolchain (v2.0 shipped, 6 crates, 13,937 LOC), and add the Contributing section with Code of Conduct and a detailed CLA. Also create CONTRIBUTORS.md.

Output: Updated README.md, new CONTRIBUTORS.md
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@README.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Recreate README.md with full project info and verbatim CLA</name>
  <files>README.md</files>
  <action>
Overwrite README.md with the following structure and content. Use the existing centered logo block at the top.

**Structure:**

1. **Logo block** (keep existing centered SVG image at top)

2. **Title and tagline:**
   `# Writ Scripting Language`
   `A game scripting language with first-class dialogue support, C-style syntax, a Rust-inspired type system, and an entity-component architecture.`

3. **Overview section** -- 2-3 paragraphs describing what Writ is. Cover:
   - Designed for game development, combining familiar scripting syntax with powerful features for interactive storytelling and gameplay logic
   - First-class dialogue system with native constructs for branching conversations, speaker management, and localization
   - Register-based IL with cooperative multitasking, entity-component system with GC, and contract-based dispatch
   - Currently a reference implementation (compiler frontend + IL runtime); AST-to-IL codegen is the next major milestone

4. **Features section** with these subsections (use ### headings, brief descriptions, not verbose):
   - First-class Dialogue Support (native `dlg` constructs, speaker resolution, localization keys)
   - Rust-inspired Type System (strong typing, `Option<T>`/`Result<T,E>`, pattern matching, explicit mutability)
   - Entity-Component Architecture (built-in entity declarations, extern-only components, lifecycle hooks)
   - Register-based IL Runtime (91-instruction VM, cooperative task scheduling, GC, contract dispatch)

5. **Project Structure section** -- list the 6 workspace crates with brief descriptions:
   - `writ-parser` -- Lexer (logos) and CST parser (chumsky)
   - `writ-compiler` -- CST-to-AST lowering pipeline (Section 28 desugaring)
   - `writ-module` -- IL binary module format (reader/writer, 200-byte header, 21 metadata tables)
   - `writ-runtime` -- Register-based VM, task scheduler, entity system, GC, contract dispatch
   - `writ-assembler` -- Text IL assembler and disassembler
   - `writ-cli` -- `writ` command-line tool (run, assemble, disassemble)

6. **Building section:**
   ```
   cargo build --workspace
   cargo test --workspace
   ```

7. **Status section:**
   - v2.0 shipped: Full IL runtime (VM, scheduler, entities, GC, contracts), binary module format, text assembler/disassembler, CLI
   - v1.x shipped: Complete CST-to-AST lowering pipeline with 7 desugaring passes
   - Next: AST-to-IL codegen
   - Link to the language spec: `language-spec/spec/` directory (NOT the old Google Doc)

8. **License section:**
   `LGPL-3.0-only`
   (Single line, matching the CLA's license reference. No LICENSE file creation in this task -- just state the license.)

9. **Contributing section** -- Insert the EXACT text provided by the user, verbatim, character-for-character. This is the text starting with `## Contributing` through the closing blockquote about the CLA statement. Do NOT modify any wording, formatting, numbering, bold markers, or whitespace patterns in this section. The user-provided CLA text is the canonical version.

The Contributing section text (copy verbatim):

```
## Contributing

Contributions are welcome!
Please submit a pull request or create a discussion to discuss any changes you wish to make.

### Code of Conduct

Be excellent to each other.

### Contributor License Agreement

By submitting a contribution (pull request, patch, or any other form) to this project, you agree
to the following terms:

1. **License Grant.** You grant the project maintainer ("Maintainer") and all recipients of the
   software a perpetual, worldwide, non-exclusive, royalty-free, irrevocable license to use,
   reproduce, modify, distribute, sublicense, and otherwise exploit your contribution under the
   terms of the GNU Lesser General Public License v3.0 (LGPL-3.0-only). You additionally grant
   the Maintainer the right to relicense your contribution under any other open-source or
   proprietary license at the Maintainer's sole discretion.

2. **Originality.** You represent that your contribution is your original work, or that you have
   sufficient rights to grant the licenses above. If your contribution includes third-party
   material, you represent that its license is compatible with the LGPL-3.0-only and permits the
   grants made herein.

3. **No Conflicting Obligations.** You represent that your contribution is not subject to any
   agreement, obligation, or encumbrance (including but not limited to employment agreements or
   prior license grants) that would conflict with or restrict the rights granted under this
   agreement.

4. **No Compensation.** Your contribution is made voluntarily and without expectation of
   compensation, unless separately agreed in writing.

5. **Right to Remove.** The Maintainer may remove, modify, or replace your contribution at any
   time, for any reason, without notice or obligation to you.

6. **Liability.** To the maximum extent permitted by applicable law, your contribution is provided
   "as is", without warranty of any kind. You shall be solely liable for any damage arising from
   the inclusion of your contribution to the extent such damage is caused by a defect, rights
   violation, or other issue originating in your contribution.

7. **Governing Law.** This agreement is governed by the laws of the Federal Republic of Germany
   (Bundesrepublik Deutschland), in particular the German Civil Code (BGB), without regard to
   its conflict-of-laws provisions. For contributors outside Germany, this choice of law applies
   to the extent permitted by the contributor's local jurisdiction.

Please add yourself to the [CONTRIBUTORS](CONTRIBUTORS.md) file when submitting your first pull
request, and include the following statement in your pull request description:

> I have read and agree to the Contributor License Agreement in this project's README.
```
  </action>
  <verify>
    <automated>grep -c "Contributor License Agreement" README.md &amp;&amp; grep -c "CONTRIBUTORS.md" README.md &amp;&amp; grep -c "LGPL-3.0-only" README.md &amp;&amp; grep -c "writ-runtime" README.md &amp;&amp; grep -c "writ-assembler" README.md</automated>
  </verify>
  <done>README.md contains: centered logo, project description, features (4 subsections), project structure (6 crates), build instructions, status with v2.0 info, LGPL-3.0-only license, and the verbatim Contributing/CLA section with all 7 numbered clauses and the CONTRIBUTORS.md link</done>
</task>

<task type="auto">
  <name>Task 2: Create CONTRIBUTORS.md</name>
  <files>CONTRIBUTORS.md</files>
  <action>
Create CONTRIBUTORS.md with the following content:

- Title: `# Contributors`
- A brief note: `Thank you to everyone who has contributed to Writ.`
- A blank line, then a note about adding yourself: `To add yourself, include your name (and optionally a link) when submitting your first pull request.`
- A blank line, then a divider `---`, then the initial contributors list with the project maintainer:
  - `- **Max Siling** -- creator and maintainer`

Use the name "Max Siling" based on the Windows user path (msili) and the German law context in the CLA. If this name is wrong, the user can trivially correct it. This is a reasonable discretion choice.

Keep the file simple and clean -- no tables, no complex formatting. Just a list that people can easily add themselves to.
  </action>
  <verify>
    <automated>test -f CONTRIBUTORS.md &amp;&amp; grep -c "Contributors" CONTRIBUTORS.md</automated>
  </verify>
  <done>CONTRIBUTORS.md exists with a title, instructions for adding yourself, and the initial maintainer entry</done>
</task>

</tasks>

<verification>
- README.md exists and contains all required sections (Overview, Features, Project Structure, Building, Status, License, Contributing with CLA)
- The CLA section contains exactly 7 numbered clauses
- The CLA text matches the user-provided version verbatim
- CONTRIBUTORS.md exists and is linked from README.md
- `grep "CONTRIBUTORS.md" README.md` returns a match
- `grep -c "^\d\." README.md` or equivalent confirms 7 CLA clauses
</verification>

<success_criteria>
- README.md accurately describes the Writ toolchain as of v2.0 (6 crates, IL runtime, assembler, CLI)
- README.md Contributing section is character-for-character identical to user-provided text
- CONTRIBUTORS.md exists with maintainer entry and instructions
- Both files are well-formatted markdown
</success_criteria>

<output>
After completion, create `.planning/quick/1-recreate-readme-and-add-github-community/1-SUMMARY.md`
</output>
