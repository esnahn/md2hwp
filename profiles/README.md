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
