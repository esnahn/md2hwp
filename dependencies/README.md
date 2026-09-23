# External dependencies

All adopted external versions and content hashes are recorded in the single
closed [`lock.json`](lock.json), validated by
[`schemas/dependencies-lock-v0.1.schema.json`](../schemas/dependencies-lock-v0.1.schema.json).
Do not create one directory or lock file per dependency.

The lock records the verified Hancom Automation environment, the official
Pandoc 3.10.1 Windows x86_64 release, and the official .NET SDK 10.0.400
Windows x64 release. rhwp remains absent until an exact revision passes
compatibility fixtures. A floating branch or guessed version is not a pin.

## Pandoc

Install or verify the pinned portable Pandoc under the ignored local dependency
tree with:

```powershell
pwsh -NoProfile -File .\dependencies\install-pandoc.ps1
```

The installer downloads the official zip, verifies its locked SHA-256, checks
the exact `pandoc 3.10.1` version line, and installs it at
`.local/dependencies/pandoc/3.10.1/pandoc.exe`. The archive is retained beside
the executable so repeat runs can revalidate the adopted payload. This script
is explicit setup and is never run by Cargo or conversion commands.

## .NET SDK

Install or verify the pinned portable .NET SDK with:

```powershell
pwsh -NoProfile -File .\dependencies\install-dotnet-sdk.ps1
```

The installer verifies the official Microsoft SHA-512 and installs the exact
Windows x64 SDK under `.local/dependencies/dotnet/<version>`. It does not modify
the system SDK or `PATH`. Invoke the pinned SDK through
`tools/development/dotnet.ps1`; that wrapper also keeps CLI state and NuGet
caches under the ignored `.local/state/` tree.

## Upstream source

Once exact compatible revisions are selected, source inspection will use two
submodules and no generic third-party source bucket:

```text
upstream/pandoc  -> https://github.com/jgm/pandoc
upstream/rhwp    -> https://github.com/edwardkim/rhwp
```

The parent repository's gitlink is the mechanical source pin. The matching
`git-submodule` entry in `lock.json` provides the auditable repository,
revision, path, and consumer; a smoke check must require both revisions to
agree once the submodules exist. Neither submodule has been materialized because
no compatible revision has been selected yet.

After they are added, a checkout retrieves both sources with the ordinary Git
command:

```powershell
git submodule update --init --recursive -- upstream/pandoc upstream/rhwp
```

Pandoc's source is for inspection only. Pandoc is a Haskell application and
md2hwp consumes its installed executable, not a Rust crate. A system-wide
alternative install command is:

```powershell
winget install --source winget --exact --id JohnMacFarlane.Pandoc
```

That command alone does not declare compatibility. Conversion still requires
the exact version adopted in `lock.json`; the repository-local installer above
is the reproducible reference path.

rhwp source is both inspectable and a candidate for a future path dependency or
CLI build. Which interface md2hwp adopts remains a backend decision. The
submodule command above is the download method; no parallel rhwp cache or source
copy is needed.

## Hancom security module

The module URL and SHA-256 remain a development-only content pin in `lock.json`
to identify the binary used for reproducible development verification. Users download and
register it by following the [official guide](https://developer.hancom.com/hwpautomation).
The application provides that URL when registration is missing or rejected.
No automatic registration or hash-based version gate is performed. The former
installer script is a retired guidance-only entry point.

The Hancom version in the lock remains a development reference, not a runtime
restriction. Pandoc and .NET SDK pins remain reproducible development inputs.
The user-facing Pandoc downloader uses the embedded Pandoc pin as its preferred
download, with latest-stable fallback; it does not require the lock file at
runtime. See [runtime dependency policy](../docs/development/runtime-dependencies.md).
