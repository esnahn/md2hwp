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

The first parser-contract fixture is
`fixtures/pandoc-json/commonmark-v0.1.json`. It is the exact JSON shape emitted
by the pinned Pandoc 3.10.1 CommonMark reader for
`examples/commonmark-v0.1.md`; the corresponding golden project IR is
`examples/commonmark-v0.1.expected.ir.json`.

The same Pandoc JSON fixture is also the direct-input CLI fixture. Its smoke
test bypasses Pandoc process invocation, requires the golden IR result, and
checks that an unsupported Pandoc API version leaves no partial output.

Every AST-to-IR ruleset needs fixtures for each declared handler and for
unlisted-constructor rejection. Equivalent backends are compared by semantic
postconditions, not by requiring identical native operation sequences.
