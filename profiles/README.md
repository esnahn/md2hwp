# Template profiles

Template profiles live below `profiles/templates/<profile>`. Each one declares
how symbolic IR styles and semantic requirements correspond to a specific
template family: style names, insertion-target selectors, required capabilities,
and layout policies.

Profiles are versioned data plus validation. They are not copies of templates,
AST-to-IR rules, backend cursor scripts, or inferred paragraph coordinates. A
backend binds a profile to the actual runtime template during preflight.

The closed v0.1 contract is `schemas/template-profile-v0.1.schema.json`.
An `investigation` profile may identify one exact tracked fixture while a
selector is being proven. It is not a production compatibility claim. A
`production` profile requires its own adopted template-family identity and
tested insertion contract.

The v0.1 Hancom marker selector is deliberately narrow: the marker must occupy
one simple root paragraph at the document end and must have one empty successor
paragraph. The backend rejects missing, duplicate, nested, unguarded, or
non-terminal markers without publishing output. It validates the terminal
paragraph structure before deletion and checks for remaining markers afterward.
