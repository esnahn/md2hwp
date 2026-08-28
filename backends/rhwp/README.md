# rhwp backend

Reserved for md2hwp's possible future backend code that uses the external
`rhwp` project or executable. It would consume the same validated IR and
template-profile semantics, but own its own template binding, lowering,
mutation, and verification logic.

The upstream source will be available as the `upstream/rhwp` Git submodule.
Its exact revision and any released executable pin must also be represented by
the single `dependencies/lock.json`; the parent gitlink and lock must agree.

The first milestone must not add a Cargo manifest, dependency, implementation,
or compatibility claim here. A later decision must choose whether this backend
uses rhwp through a Rust path dependency or its CLI.
