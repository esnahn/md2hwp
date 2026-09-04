# AGENTS.md

## Goal

Build a deterministic converter from a small, explicit shared subset of
Pandoc-normalized CommonMark and reStructuredText to Korean Hangul documents.

```text
CommonMark / reStructuredText -> Pandoc JSON + versioned AST2IR rules
                              -> core -> validated project IR
                              + template profile + required HWP/HWPX template
                              -> backend-specific lowering -> output -> verification
```

Template fidelity and Hancom compatibility matter more than Markdown parsing.
Keep parsing, document generation, and verification separate.

## Current state

- The repository contains AURI reference assets under `reference/`, exploratory
  Hancom Automation scripts under `tools/`, and generated proof documents under
  `artifacts/`.
- `tools/investigation/auri/build-format-examples.ps1` is a
  template-investigation script,
  not the converter or a production backend. Its fixture-specific paragraph
  coordinates document what was observed in the current AURI sample only.
- There is no production converter yet. The workspace now has closed IR v0.1
  types, strict validated I/O, semantic/resource validation, and Rust unit
  tests. The contract smoke test validates the AST-to-IR ruleset, the external
  dependencies lock, and accepted/rejected IR schema examples; there is no
  HWP/HWPX converter or Hancom end-to-end smoke suite yet.
- The canonical core implementation language is Rust. The shared crate is
  `crates/md2hwp-core`, and the user-facing application boundary is
  `apps/md2hwp`. The workspace pins Rust 1.98.0 for
  `x86_64-pc-windows-msvc`, uses edition 2024 and Cargo resolver 3, and sets
  the initial MSRV to Rust 1.98.0. `apps/md2hwp` now exposes CommonMark source
  and direct Pandoc JSON input modes for IR output, and `crates/md2hwp-core`
  implements the pinned Pandoc JSON input contract, built-in AST2IR rules, and
  normalization handlers.
- The Hancom Automation worker implementation language is C#/.NET under
  `backends/hancom-automation`. The .NET SDK, target framework, project files,
  and COM interop details are not declared yet.
- Do not impose repository-wide `src/`, `bin/`, `dist/`, or `share/` roots.
  Rust packages use Cargo's package layout, C# projects use .NET project and
  test-project conventions, and declarative assets remain in domain-named
  `rules/`, `schemas/`, and `profiles/` trees. Generated `target/`, `bin/`, and
  `obj/` directories are compiler output, not repository boundaries.
- All adopted external pins live in the single `dependencies/lock.json`.
  Pandoc process invocation stays in `apps/md2hwp`; backend integration stays
  under `backends/`. Once Git and compatible revisions are selected, upstream
  Pandoc and rhwp sources are pinned submodules at `upstream/pandoc` and
  `upstream/rhwp`. Do not create parallel reader, per-dependency metadata, or
  generic imported-source directory trees. Downloaded executable/archive
  payloads must use an explicitly documented ignored path or an explicitly
  documented external install location; do not track them.
- Product-owned AST-to-IR policy is data in
  `rules/ast2ir/ir-v0.1.json`, validated by its closed schema. The core owns the
  finite handler implementations; the ruleset is not executable code or an
  implicitly replaceable user configuration file.
- The initial reference environment is Hancom Office 2020 HWP 11.0.0.9136,
  registered as `HWPFrame.HwpObject`.
- The official file-access security DLL is not tracked in the repository.
  `dependencies/install-hancom-security-module.ps1`
  reuses a matching ignored local copy or downloads the official archive,
  verifies the URL/hash from the closed `dependencies/lock.json`, installs the
  workstation-local copy at the exactly ignored
  `dependencies/FilePathCheckerModuleExample.dll`, and registers it under
  `HKCU\Software\HNC\HwpAutomation\Modules`.
- Before downloading or writing anything, that installer must inspect the
  current user's existing REG_SZ registration. An absolute, present DLL whose
  SHA-256 matches `dependencies/lock.json` is authoritative regardless of its
  install directory: validate it with Hancom and return `AlreadyValid` without
  relocating or rewriting it. If Hancom rejects that otherwise-valid
  registration, fail without persistent mutation. Only a missing, malformed,
  missing-file, or hash-mismatched registration may enter transactional repair.
  During repair, reuse the ignored local DLL without downloading when its hash
  matches the content pin; otherwise download and verify the official archive.
- Do not invent a toolchain, command, API, or compatibility claim that is not
  supported by the repository or the installed reference environment.

## First milestone

- Use Pandoc JSON as the parser output contract. CommonMark and
  reStructuredText readers must normalize only their explicitly adopted shared
  subset to project IR.
- Use an explicit syntax allowlist. Unsupported nodes must fail with the Pandoc
  constructor and AST path; never silently discard them.
- Keep the allowlist and simple mappings in a versioned product-owned data
  ruleset. Core handlers remain closed typed code; rules must never contain or
  invoke arbitrary code.
- Require an existing Korean report template. Do not create a production
  document from a blank file.
- Initially target the AURI basic-research report while keeping style mappings
  configurable for other templates.
- Preserve Unicode exactly unless an explicit policy says otherwise.

Supported content:

- paragraphs and headings;
- spaces, soft breaks normalized to spaces, and explicit line breaks;
- nested strong and emphasis;
- links, preserving label, target, and optional title in IR;
- native HWP bullet and numbered lists;
- line-preserving verbatim blocks;
- figures supplied as trusted direct IR, using an explicit template-specific
  figure, caption, and source-line mapping. CommonMark/reStructuredText figure
  normalization remains disabled until its syntax is defined.

Deferred to a later IR version:

- generated tables;
- generated footnotes and endnotes;
- page breaks.

Rejected in IR v0.1:

- inline code and block quotes;
- raw blocks and raw inlines;
- horizontal rules;
- any constructor not explicitly supported.

Existing tables, figures, headers, footers, styles, page setup, backgrounds, and
unrelated template content must still be preserved.

## Content and style rules

- Keep project styles symbolic: `body`, `heading.1`, `block.box`, and so on.
- Resolve symbolic styles through a template-specific profile only inside each
  backend's template-binding and lowering layers.
- In the AURI basic template, `block.box` initially maps to `박스내용`.
- Anchor AURI boxes in a `본문` paragraph; apply `박스내용` only to their
  internal content.
- A verbatim block is general-purpose; do not encode it as a law-specific type or
  render it as monospaced source code.
- Insert AURI figures as inline characters in a `본문` paragraph. Preserve the
  source aspect ratio and limit width to 142 mm. Put the caption next, followed
  immediately by a source placeholder; source metadata may later come from a
  defined CommonMark or reStructuredText convention.
- Treat the current empty source line as a placeholder. Define the CommonMark
  and reStructuredText source-metadata syntax and its Pandoc/IR mapping before
  production use.
- Lists use the template's `body` text style plus Hancom's native bullet or
  paragraph-numbering feature. Do not insert literal list markers or reuse
  heading outline styles.
- The AST-to-IR ruleset maps Pandoc `SoftBreak` to IR `Space` during
  normalization. `SoftBreak` is not an IR node. `LineBreak` remains a line break
  in the same paragraph and is neither a paragraph nor a page break.
- Preserve `Strong` and `Emph` as nested semantic inline nodes. Flatten them
  only inside each backend's lowering to adjacent text runs with an active mark
  set. Apply the marks as character-shape patches over the resolved base style.
- Preserve link label, target, and optional title in IR. The AURI v0.1 profile
  intentionally renders the recursively formatted label only.
- Do not reparse Markdown after Pandoc or leave Markdown markers in HWP text.

The normative serialized IR v0.1 contract is in
`docs/specifications/ir-v0.1.md`, with its
machine-readable closed schema in `schemas/ir-v0.1.schema.json`. Its stable
summary is:

```text
Document { schema, ir_version, metadata, blocks }

Block =
  Paragraph { inlines }
  Heading { level, inlines }
  VerbatimBlock { lines }
  List { kind, start?, tight, items }
  Figure { image, caption, source }

Inline = Text | Space | LineBreak | Strong | Emph | Link
```

IR v0.1 metadata is an empty object. Tables, footnotes, page breaks, inline
code, block quotes, raw nodes, horizontal rules, and unlisted constructors are
not representable. Figures are representable, but their source-language syntax
and source-metadata mapping remain open decisions.

## Architecture

Keep these layers distinct:

1. `apps/md2hwp`: when source text is supplied, invoke the exact declared
   Pandoc executable with explicit reader and JSON-writer options and capture
   stdout/stderr; direct Pandoc JSON bypasses process invocation. Keep this a
   small application module rather than a separate reader package until reuse
   justifies another boundary.
2. `pandoc_input`: decode and version-check Pandoc JSON.
3. `ast2ir_rules`: load and validate the exact product-owned ruleset.
4. `normalize`: apply closed rule handlers and convert Pandoc nodes to project
   IR.
5. `ir`: own the closed project types.
6. `ir_io`: serialize only validated IR and deserialize UTF-8 JSON by checking
   the envelope, exact version, closed JSON Schema, and typed representation.
7. `validate`: enforce IR-local semantic invariants and configured resource
   limits.
8. `template_profile`: declare symbolic mappings, target selectors, layout
   policy, and required capabilities without native coordinates or handles.
9. Per backend: inspect and bind the actual template, lower validated IR into
   the backend's native model, mutate a template copy, and verify the result.

The normalized-source path is:

```text
source -> apps/md2hwp -> Pandoc CLI -> pandoc_input + ast2ir_rules
direct Pandoc JSON ------------------> pandoc_input + ast2ir_rules
                                      -> normalize -> ir -> validate
                                                       -> ir_io(write)?
```

A serialized-IR replay enters through:

```text
ir_io(read) -> ir -> validate -> selected backend
```

No IR file is written before validation succeeds. IR is the only public,
persistent intermediate document format. A separately launched backend may use
a small versioned invocation envelope to transport validated IR, profile
identity, template/output paths, and options, but that envelope is not another
public document representation.

Pandoc types and AST-to-IR rule handlers must not reach a backend. Each backend
receives validated IR and owns its lowering implementation. COM cursor,
selection, paragraph, and character coordinates must not escape the Hancom
adapter. Equivalent backends share semantic postconditions and conformance
fixtures rather than native lowering code. Keep a peer boundary for a future
`rhwp` adapter, but do not implement or depend on `rhwp` in the first milestone.
If adopted later, `backends/rhwp` contains md2hwp's integration code,
`upstream/rhwp` contains the pinned upstream submodule, and
`dependencies/lock.json` records the matching revision or release artifact.

## Template and backend rules

- Treat a template profile and a runtime HWP/HWPX file as separate inputs. The
  profile declares intended meanings; the backend must inspect the actual file,
  validate required styles/capabilities, and bind each declared selector and
  mapping without guessing.
- Launch the Hancom Automation backend as a separate worker process so COM
  crashes, hangs, runtime/bitness concerns, and cleanup remain outside the core
  process. The exact internal invocation protocol remains an open decision.
- Use Hancom's documented OLE/COM API. Do not automate mouse clicks, keystrokes,
  or screen coordinates.
- Immediately after creating `HWPFrame.HwpObject` and before opening a file,
  require
  `RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")` to
  return true.
- Run in a logged-in interactive Windows session and process one document at a
  time. Headless, service, scheduled, and concurrent execution are out of scope.
- On the reference workstation, all Hancom COM investigation, security-module
  installation or re-registration, and backend verification MUST use the
  verified execution context in `docs/development/environment.md`. HKCU and
  `%LOCALAPPDATA%` are per-user: never use another account's, sandbox's, or
  noninteractive process's result to declare the module present or absent. A
  different context is not adopted until the canonical open-only probe passes
  there and the environment record is updated.
- Installation and re-registration are user-only operations. Agents MUST NOT
  invoke `dependencies/install-hancom-security-module.ps1`, including through
  sandbox escalation or another approved command-runner context. When either
  operation is needed, tell the user to run the exact command documented in the
  root `README.md` from their own logged-in interactive Windows PowerShell 5.1
  window and return its complete output. Agents may inspect that output but
  must not perform the HKCU or managed-DLL mutation themselves.
- An agent may run the canonical open-only probe only in an explicitly approved
  non-default context after confirming that the actual Windows process token,
  interactive session, and PowerShell host match the verified environment. A
  result from the default sandbox is not workstation verification.
- Never blindly accept security, overwrite, repair, compatibility, or data-loss
  prompts. Clean up documents and COM processes after success or failure.
- Never modify the source template. Write to a separate output path.
- Delete whole paragraphs or ranges by selecting from the current paragraph
  beginning through the next paragraph beginning (`MoveSelNextParaBegin`). Do
  not combine `SetPos(..., 0)`, `SelectPara`, and `Delete`; in HWP 2020 that
  reproduced loss of the next paragraph's first character. Verify the complete
  text immediately before and after every deleted range.
- Resolve a declared, unique insertion target. Prefer named fields/bookmarks,
  then a unique marker, then a structurally identified paragraph. Never guess
  when multiple candidates exist.
- Hard-coded paragraph coordinates are permitted only in clearly labeled,
  fixture-specific investigation scripts. Do not carry them into the converter
  or production backend.
- Fail before writing output when the template or insertion target is missing,
  unreadable, unsupported, malformed, or ambiguous.
- Preserve the template format: HWP to HWP and HWPX to HWPX. Do not claim the
  formats are interchangeable.

## Verification and completion

Use fixture-driven tests for:

- normalization of every supported constructor and rejection of unsupported
  nodes, including `SoftBreak` to IR `Space`, Korean, and mixed Unicode text;
- schema and typed validation of every product-owned AST-to-IR ruleset;
- closed-schema validation of the single external dependencies lock;
- missing and ambiguous templates without partial output;
- paragraph, heading, list, verbatim-block, and line-break structure;
- direct-IR embedded figures, aspect ratio, inline placement, caption
  numbering, and source-line placement;
- preservation of unrelated template content;
- reopening the output in the target Hancom version;
- visual checks for pagination, fonts, spacing, lists, and styled boxes.

Text extraction alone is not sufficient. The first milestone is complete only
when a documented command converts CommonMark, reStructuredText, or Pandoc
JSON into an edited template, rejects unsupported syntax clearly, preserves
surrounding content, and passes structural and visual verification.

## Open decisions

- .NET SDK, target framework, solution/project files, and COM interop strategy;
- Hancom Automation licensing;
- internal backend invocation envelope and process protocol;
- template insertion contract;
- full AURI symbolic style mapping;
- verbatim-block presentation policy;
- CommonMark/reStructuredText figure/source syntax and Pandoc-to-IR mapping;
- page-break representation in a later IR version;
- initial HWP/HWPX support range and visual comparison method.

Resolve an open decision in documentation and tests before relying on it. When
blocked, report the alternatives rather than choosing silently.

## Start of a fresh session

Read this file, inspect the repository, confirm the installed Hancom/COM
environment and security setup from the verified context in
`docs/development/environment.md`, then run the canonical HWP open-only probe
there and the smallest repository contract smoke test. If a prerequisite or
test does not exist, report that instead of inventing it.
