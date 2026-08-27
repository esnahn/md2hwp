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

This directory is a Cargo binary package in the root workspace. The initial
functional slice will accept serialized, validated project IR through an
explicit `--from ir` mode; source parsing through Pandoc follows later. The
current `src/main.rs` is deliberately compile-only and does not expose a CLI
contract yet.
