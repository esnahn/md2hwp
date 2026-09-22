# ADR 0009: CommonMark object sources

- Status: accepted for implementation
- Date: 2026-09-23

Use an immediately following, separate paragraph beginning with the exact plain
text `출처:` followed by a space and nonempty inline content as object metadata.
Apply one shared AST-level attachment handler to top-level figures and verbatim
blocks. A source prefix elsewhere remains ordinary prose. Never reparse Markdown.
Only one adjacent source paragraph is consumed; later paragraphs remain prose.
An empty reserved source paragraph after an eligible object is an error.

A top-level CommonMark paragraph containing exactly one image becomes Figure.
Its alt inlines become both caption and image alt; the optional image title is
preserved as image.title. Empty captions, inline/multiple images, images in lists,
attributes, remote/absolute paths and unsupported caption/source nodes are
rejected. Image paths are relative to the input Markdown or Pandoc JSON file;
the application rebases them relative to its IR output without fetching files.
The backend enforces its existing repository resource boundary. Relative `..`
segments remain allowed by the IR contract.

The pinned invocation remains `--from=commonmark --to=json`. Observed Pandoc
3.10.1 emits Para(Image), then Para(Str("출처:"), Space, ...). The same restricted
AST can be replayed through direct Pandoc JSON. Native Pandoc Figure and Table
constructors are not implicitly enabled.

IR 0.2 adds optional nullable `source` inlines to VerbatimBlock. Missing and null
both mean absent; serialization omits an absent box source. All other IR shapes
remain unchanged. Readers preserve and validate 0.1 against its original closed
schema and reject box source fields there. New normalization targets 0.2 through
ruleset 0.2. Prior rules/schema assets remain historical fixtures.

HWP uses each object's template source paragraph; it adds the template-owned
label exactly once, retaining inline marks and the existing label-only behavior
when metadata is absent. Figure numbering remains native. Source attachment
does not create an extra prose paragraph.

Tables use this same adjacent-source convention when table normalization is
adopted. This convention alone does not enable table parsing or generation.
Table support needs its own IR structure, template prototype and tests; it must
not silently render a pipe table as a generated HWP table. reStructuredText
figure/table syntax remains a separate unimplemented reader decision.
