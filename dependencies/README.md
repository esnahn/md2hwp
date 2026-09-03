# External dependencies

All adopted external versions and content hashes are recorded in the single
closed [`lock.json`](lock.json), validated by
[`schemas/dependencies-lock-v0.1.schema.json`](../schemas/dependencies-lock-v0.1.schema.json).
Do not create one directory or lock file per dependency.

The lock records the verified Hancom Automation environment and the official
Pandoc 3.10.1 Windows x86_64 release. rhwp remains absent until an exact
revision passes compatibility fixtures. A floating branch or guessed version
is not a pin.

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

The explicit installer reads the Hancom entry from `lock.json`:

```powershell
C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\dependencies\install-hancom-security-module.ps1
```

It installs the exactly ignored `dependencies/FilePathCheckerModuleExample.dll`
and mutates the current user's registry, so it is never a build or test side
effect and the DLL is never committed. It must run in the verified Windows
identity/profile and interactive Windows PowerShell context recorded in
`docs/development/environment.md`. An existing REG_SZ registration that points
to an absolute, present DLL with the pinned hash is authoritative regardless of
its install directory. The installer validates it before any download or write
and preserves it as `Action=AlreadyValid`; a Hancom rejection at that point
also fails without mutation. With an invalid or missing registration, the
installer first reuses the ignored managed DLL when its hash matches the content
pin. It downloads and verifies the official archive only when that local copy
is absent or invalid. A failed fallback restores the previous file and registry
value. Any other downloaded executable/archive payload added later requires its
own explicitly documented ignored destination.

The HKCU value points directly to the ignored DLL in this working tree. After
moving or deleting the repository, or after a cleanup that removes ignored
files such as `git clean -fdx`, re-run the installer from the repository's new
or restored location before using HWP Automation.
