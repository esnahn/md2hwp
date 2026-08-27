# Applications

User-facing deployable entry points live here. An application may be a Cargo
workspace member, but reusable canonical logic belongs in `crates/` and native
document implementations belong in `backends/`.

Rust applications use their own Cargo manifests and standard package-local
`src/main.rs`; `apps/` is not a generic source directory for every language.
