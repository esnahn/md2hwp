# Schemas

Machine-readable contracts live here under semantic, version-bearing names.
Their `$id`, declared draft, compatibility policy, and consumers determine
identity; they are not programming-language source files and do not move under
a generic `src/` or `share/` directory.

Generated Rust or C# bindings, if introduced later, belong to the package or
project that compiles them and must record the generator version. The canonical
JSON Schemas remain here.

`dependencies-lock-v0.1.schema.json` defines the single closed manifest at
`dependencies/lock.json`. A dependency absent from that manifest is not yet
adopted; floating or guessed values must not be represented as pins.

`template-profile-v0.1.schema.json` defines backend-facing symbolic style,
selector, layout, capability, and template-identity declarations. The initial
consumer is an explicitly non-production AURI minimal-fixture investigation
profile.
