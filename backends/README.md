# Backends

Backends consume validated project IR, a versioned template profile, and the
runtime template/output paths. Each backend owns template inspection and
binding, backend-specific lowering, document mutation, and structural/visual
verification.

Equivalent backends share IR semantics, profile vocabulary, fixtures, and
semantic postconditions. They do not share their native lowering
implementation. A backend's cursor, selection, package, and object model cannot
escape its adapter boundary.

Each backend follows its implementation ecosystem rather than a common source
tree. The initial Hancom Automation worker uses C#/.NET; a future backend may
choose a different language without moving behind a language-named top-level
directory.
