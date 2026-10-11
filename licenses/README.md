# Retained third-party license material

These files retain their upstream licenses and copyright notices. The project's
AGPL declaration does not relicense them. Ship this directory together with
`THIRD-PARTY-NOTICES.txt`.

- `rust-1.98.0/COPYRIGHT-library.html` and its `licenses/` directory are copied
  verbatim from the installed, pinned Rust 1.98.0 distribution's
  `share/doc/rust/`. This upstream standard-library inventory includes material
  for other targets and configurations. Source:
  https://github.com/rust-lang/rust/tree/1.98.0
- `dotnet-apphost-10.0.11/LICENSE.TXT` and `THIRD-PARTY-NOTICES.TXT` are copied
  verbatim from the official `Microsoft.NETCore.App.Host.win-x64` 10.0.11 NuGet
  package. Its package metadata declares MIT; its `singlefilehost.exe` matches
  the host in the pinned .NET 10.0.400 SDK byte for byte. Source revision:
  https://github.com/dotnet/dotnet/tree/e2f47b0110ed922f21a1522da67279133ce28f32
  Package:
  https://www.nuget.org/packages/Microsoft.NETCore.App.Host.win-x64/10.0.11
- `supplemental/jsonschema-0.18.3-LICENSE.txt` comes from the exact commit recorded
  in that crate's `.cargo_vcs_info.json`. The crate declares MIT but omitted its
  license text from the published archive. Original:
  https://raw.githubusercontent.com/Stranger6667/jsonschema/11ebbaaececc1830f093dca229a28c0a5ebdf877/LICENSE

The locked Cargo package notices are embedded in the root
`THIRD-PARTY-NOTICES.txt`, generated from the exact installed crate versions and
the supplemental file above by `tools/development/update-third-party-notices.ps1`.
Supplemental Unicode-table and adapted-code notices are retained alongside the
main package license texts. Where multiple licenses are offered, the file states
the selected permissive alternative without removing the upstream attribution.

When upgrading dependencies or toolchains, refresh the exact upstream evidence,
regenerate the root notices, and review new terms. Do not replace copyright
holders or years with the md2hwp author's details. Official SDK product license
terms and the separately licensed host package must not be conflated.
