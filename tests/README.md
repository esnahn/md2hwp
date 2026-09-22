# Tests

Use these subtrees as the suite grows:

- `fixtures/source/{commonmark,rst}` for source-language inputs;
- `fixtures/pandoc-json` for parser-contract inputs;
- `fixtures/ir` and `fixtures/assets` for direct IR jobs;
- `fixtures/templates` only for provenance-cleared template fixtures;
- `expected` for deterministic IR, diagnostics, and semantic postconditions;
- `integration/hancom-automation` for serialized interactive Windows tests;
- `visual` for reviewed baselines after the comparison method is decided.

Generated output belongs in `artifacts/`, never beside immutable fixtures.
Examples may be reused by tests, but a test should not mutate them.

`fixtures/template-declarations/explicit-ranges.txt` models an ordered stream
of paragraph text for the experimental template declaration scanner. It is not
an HWP fixture or a serialized public document format. The C# console harness
under `tools/investigation/hancom-automation/template-declarations-tests` checks
explicit bounds and slot ownership plus malformed mutations of that fixture.
See the [investigation draft](../docs/design/template-declarations-investigation.md)
for its command and the native/visual checks still required.

`fixtures/templates/minimal.hwp` is identified by exact byte length and SHA-256
in the non-production AURI investigation profile. Contract smoke validates the
closed profile and identity, while the C# preview smoke verifies that a changed
template is rejected before COM and leaves no output.

The first parser-contract fixture is
`fixtures/pandoc-json/commonmark-v0.1.json`. It is the exact JSON shape emitted
by the pinned Pandoc 3.10.1 CommonMark reader for
`examples/commonmark-v0.1.md`; the corresponding golden project IR is
`examples/commonmark-v0.1.expected.ir.json`.

The same Pandoc JSON fixture is also the direct-input CLI fixture. Its smoke
test bypasses Pandoc process invocation, requires the golden IR result, and
checks that an unsupported Pandoc API version leaves no partial output.

`fixtures/ir/two-boxes-v0.1.json` is the repeated native-box investigation
fixture. Both operations intentionally use text identical to the template
prototype, exercising insertion identity when adjacent roots serialize alike.
The non-COM preview smoke test requires two independent typed box operations.
On the adopted HWP 2020 workstation it is also the manual render input for
checking two successive prototype clones and their reopened logical content.

`fixtures/ir/two-figures-v0.1.json` repeats the minimal template's exact
caption text with the same image. It exercises adjacent prototype-identical
caption clones: the non-COM plan must retain two independent figure operations,
and the adopted HWP 2020 render must reopen with two new native Figure
automatic-number controls in sequence.

`fixtures/templates/minimal-marker.hwp` is a repository-owned comparison
fixture prepared from `minimal.hwp`. It adds one dedicated
`{{MD2HWP_INSERTION_TARGET_V0_1}}` paragraph followed by an empty paragraph.
The paired investigation profile must remove that whole marker paragraph before
rendering and verify the complete root-paragraph text sequence immediately
after deletion.

Treat `minimal.hwp` and `minimal-marker.hwp` as the primary visual comparison
pair. The first is the untouched small example and the second is the render
input; the full external AURI report is not substituted for either fixture.
Visual verification must confirm that the marked copy preserves the example
body with only the insertion marker added. This has not yet been verified for
the current double-curly-brace marker fixture.

Every AST-to-IR ruleset needs fixtures for each declared handler and for
unlisted-constructor rejection. Equivalent backends are compared by semantic
postconditions, not by requiring identical native operation sequences.

## Tagged minimal template

`fixtures/templates/minimal-tagged-v1.hwp` is the authored, usable `minimal-1`
template. Its provenance, exact tags, render commands, native verification,
and scope are recorded in
[`minimal-tagged-template.md`](../docs/development/minimal-tagged-template.md).
`fixtures/ir/tagged-template-conformance-v0.1.json` exercises same-paragraph
line breaks in body/list text, repeated boxes, native heading markers, and
literal tag-like manuscript text. The contract smoke validates this IR; the
native results and page inspection are documented separately from COM-free
tests.
