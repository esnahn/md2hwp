# ADR 0008: Template-owned bindings

- Status: accepted direction; native representation pending investigation
- Date: 2026-09-22

## Decision

The intended render interface takes validated project IR and an authored HWP
template, plus the output destination. Users should not maintain a separate
template-profile JSON file. External image resources referenced by IR remain
required; this is not a decision to embed those resources in IR.

The template owns its insertion target, symbolic role declarations, native
styles, reusable structures, and template-specific layout settings. A backend
inspects these declarations and builds a validated internal binding before
editing a copy. IR remains backend-neutral and its v0.1 contract is unchanged.

This supersedes the requirement for a separately supplied production profile.
It does not remove template inspection, semantic style roles, validation, or
backend-specific lowering. Existing investigation profiles remain unchanged
until the replacement contract is proven and implemented.

## Ownership and authoring

The [explicit-range investigation draft](../design/template-declarations-investigation.md)
distinguishes insertion points, paragraph samples, paired prototype boundaries,
and owner-scoped content slots. Its lexical tests do not adopt a native HWP
representation or establish render compatibility.

Declarations identify roles rather than executable expressions. The closed,
versioned template contract must distinguish:

- the unique content insertion target;
- sample paragraphs for body and supported heading styles;
- complete reusable structures for boxes, lists, and figures;
- figure caption and source roles within their owning structure;
- settings that cannot be inferred from formatting, such as the figure width
  limit, source-line policy, and supported nesting depth;
- an explicitly bounded authoring-sample area, removed only from the output
  copy after its structures have been captured.

Paragraph samples supply their native style bindings. Box and figure samples
must identify the complete structure and its content slots, not merely a
paragraph style. Strong and emphasis remain character-shape patches over the
resolved base style; list numbering remains native and IR controls its start.

Plain-text markers such as `{{md2hwp:body}}` are illustrative, not an adopted
syntax. Compare dedicated text markers with named Hancom fields/bookmarks in
the reference environment before choosing their native representation. Define
scope, cardinality, range boundaries, and safe removal in that investigation.
Do not assume that replacing field text can insert arbitrary document blocks.

## Validation and migration

Validate the contract version, required roles for the supplied IR, unique
declarations, style availability, prototype structure, settings, and backend
capabilities before publishing output. Reject unknown declarations, ambiguous
targets, malformed sample ranges, and unsupported structures without guessing.
User content that resembles a tag must not be interpreted as a declaration.

Production acceptance should depend on the declared contract and inspected
structure, not an allowlist of whole-file hashes. Record hashes for provenance
and input-preservation checks. Existing exact-hash investigation fixtures stay
exact-hash fixtures during migration.

Define the worker invocation contract after the template declaration
investigation establishes the required inputs. The external-profile invocation
proposal from the discarded development branches is not an adopted baseline
and creates no compatibility obligation. Process isolation, validated IR
replay, resource boundaries, and output verification remain required.

## Evidence required before implementation relies on this decision

1. Create a separate authored minimal fixture; preserve the current fixtures.
2. Compare native fields/bookmarks and text markers for role discovery, exact
   range capture, save/reopen persistence, and unambiguous removal.
3. Prove body and heading style inheritance, native list behavior, box cloning,
   and inline figures with native caption numbering and adjacent source lines.
4. Prove missing, duplicate, unknown, malformed, and unsupported declarations
   fail without output; test tag-like manuscript text as ordinary content.
5. Verify full surrounding text and structure, same-paragraph line breaks,
   sample-area removal, original-template hashes, and process cleanup.
6. Reopen outputs in the reference Hancom version and visually compare all
   pages for styles, spacing, pagination, lists, boxes, and figures.
7. Adopt the precise declaration schema and worker invocation contract with
   fixtures and tests before integrating a production render command.

Live COM investigations must follow the approved interactive context in
`docs/development/environment.md`. This decision records no new live result
and does not claim HWPX compatibility.
