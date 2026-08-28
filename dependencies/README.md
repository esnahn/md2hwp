# External dependencies

All adopted external versions and content hashes are recorded in the single
closed [`lock.json`](lock.json), validated by
[`schemas/dependencies-lock-v0.1.schema.json`](../schemas/dependencies-lock-v0.1.schema.json).
Do not create one directory or lock file per dependency.

The lock currently records only the verified Hancom Automation reference
environment and security-module archive. Pandoc and rhwp are intentionally
absent until an exact release/revision passes the repository's compatibility
fixtures. A floating branch or guessed version is not a pin.

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
md2hwp consumes its installed executable, not a Rust crate. The official simple
Windows install command is:

```powershell
winget install --source winget --exact --id JohnMacFarlane.Pandoc
```

That command alone does not declare compatibility. Production conversion must
also require the exact Pandoc version adopted in `lock.json`; add a pinned
installer or verifier when that version is selected.

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
also fails without mutation. Only an invalid or missing registration enters the
transactional fallback, which downloads and verifies the official archive
before replacing the managed DLL and registration. A failed fallback restores
the previous file and registry value. Any other downloaded executable/archive
payload added later requires its own explicitly documented ignored destination.

The HKCU value points directly to the ignored DLL in this working tree. After
moving or deleting the repository, or after a cleanup that removes ignored
files such as `git clean -fdx`, re-run the installer from the repository's new
or restored location before using HWP Automation.
