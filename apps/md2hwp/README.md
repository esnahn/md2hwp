# md2hwp application

Reserved for the user-facing command-line application. It will invoke the
declared Pandoc executable with explicit reader and JSON-writer options, pass
Pandoc JSON to `md2hwp-core`, and
invoke the selected backend with validated IR, template-profile identity,
template path, output path, and execution options. The Hancom Automation
backend is launched as a separate worker; another adopted backend may use a
different boundary after that decision is documented.

Pandoc process launch, version checking, stdout/stderr capture, and exit-code
propagation stay as a small application module until reuse proves that another
package is warranted. The application does not implement AST normalization,
template interpretation, or backend lowering. Its internal worker protocol
remains an open decision.

This directory is a Cargo binary package in the root workspace. Serialized,
validated project IR replay remains planned through an explicit `--from ir`
mode.

The first implemented command converts the adopted CommonMark subset to
validated IR through the pinned Pandoc executable:

```powershell
cargo run -p md2hwp -- md2ir `
  --from commonmark `
  --input .\examples\commonmark-v0.1.md `
  --output .\artifacts\commonmark-v0.1.ir.json
```

It checks the exact Pandoc 3.10.1 version, captures JSON/stdout and diagnostics,
normalizes only allowlisted constructors through the built-in ruleset, and
writes only validated IR. Existing output requires `--force`. Backend
invocation and serialized-IR replay remain unimplemented.

To start from an existing Pandoc JSON file, select `pandoc-json`. This path does
not locate, launch, or version-check a Pandoc executable:

```powershell
cargo run -p md2hwp -- md2ir `
  --from pandoc-json `
  --input .\tests\fixtures\pandoc-json\commonmark-v0.1.json `
  --output .\artifacts\pandoc-json-v0.1.ir.json
```

The JSON envelope and API version are still checked before the same rules,
normalization handlers, semantic validation, and validated-only writer run.
`--pandoc` is rejected in this mode rather than silently ignored.
