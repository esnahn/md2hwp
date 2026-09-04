# Tools

- `development/`: read-only environment diagnostics and contributor helpers.
- `investigation/`: exploratory or fixture-specific scripts that are not
  production backends.
- `smoke/`: the smallest repeatable repository checks.

Production code must not import from `tools/`.
Dependency setup reads the consolidated `dependencies/lock.json`. The
specialized Hancom installer lives directly under `dependencies/`, not here;
such scripts remain explicit operations and must never run as an implicit
build/test side effect.

`investigation/test-hwp-open.ps1` is the canonical open-only Hancom probe. It
must be invoked with the verified Windows PowerShell context documented in
`docs/development/environment.md`. It checks the registered DLL against the
locked hash, requires per-object module registration, opens an existing HWP,
and closes it without saving. A nonzero exit means the environment is not ready
for further HWP work.

`smoke/test-commonmark-to-ir.ps1` runs the pinned Pandoc 3.10.1 through the
Rust CLI, validates the generated IR, compares it with the golden example,
checks the documented CommonMark Unicode composition observation, and proves
that unsupported syntax leaves no partial output.

`smoke/test-pandoc-json-to-ir.ps1` enters through the direct Pandoc JSON CLI
path without launching Pandoc, compares the result with the same golden IR,
and proves that an unsupported Pandoc API version leaves no partial output.

`development/dotnet.ps1` invokes the pinned repository-local .NET SDK with
repository-local CLI and NuGet state. The C# investigation project under
`investigation/hancom-automation/ir-preview` turns validated IR into an
explicit preview plan and contains manually gated open-only/render COM modes.
`smoke/test-hancom-ir-preview-plan.ps1` builds it and validates the full IR
example without invoking COM.
