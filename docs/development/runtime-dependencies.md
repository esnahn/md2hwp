# Development pins and user prerequisites

Adopted 2026-09-23 at the user's direction. This policy supersedes the old
hash-pinned Hancom installer and exact Pandoc release checks in historical notes.

`dependencies/lock.json` now records Pandoc and .NET SDK build/development pins,
plus the tested Hancom application version as a reference. Hancom's security
module URL and SHA-256 are retained as a development content pin (restored
2026-09-24). They identify the tested binary for reproducible development
verification and are not enforced by user runtime or used for automatic setup.
Rust's toolchain remains in `rust-toolchain.toml`.
None of these exact development versions is a blanket user update prohibition.

## Hancom

Users follow <https://developer.hancom.com/hwpautomation> to download the
Automation module and register it in their own account. The application never
downloads it, writes registration, or compares it against a fixed DLL hash.
It still requires a REG_SZ registration, an existing absolute DLL path, and
successful `RegisterModule("FilePathCheckDLL", "FilePathCheckerModuleExample")`
as the first COM call before opening a document. Failure includes the official
setup URL. Observed hashes are diagnostic only. Existing registrations/files
are left untouched. The retired installer entry point only prints guidance.

Tagged rendering no longer searches for a repository or reads a dependency
lock. Its resource root is the invocation's working directory; relative IR
image paths must resolve inside that directory. Run from the project containing
the manuscript/resources. Legacy fixture/profile investigation modes still
require the development repository.

## Pandoc

`md2hwp setup-pandoc` explicitly downloads upstream's portable Windows x64 ZIP.
Download, SHA-256 verification and ZIP extraction run in Rust, without a
PowerShell subprocess. HTTPS uses Windows TLS and the operating system trust
store. The command always performs a download when explicitly invoked; normal
conversion only searches for an existing executable and never installs one.
The preferred version/URL/archive digest are embedded from the development lock
at build time, so no lock file is shipped or read from disk at runtime. If that
download is unavailable, the downloader queries the official GitHub latest
stable release API and uses that release's Windows x64 ZIP and SHA-256 digest.
An integrity failure stops installation; it does not silently switch sources.

Downloads go under `%LOCALAPPDATA%\md2hwp\pandoc\<version>-<unique-id>\`.
The whole ZIP, its documentation, release-specific COPYRIGHT, and an UPSTREAM.txt
with source/release URLs are retained. No MSI is executed and system PATH is not
changed. A current.txt pointer is replaced only after extraction and CommonMark
JSON compatibility checks succeed. Existing installations remain intact on failure.
If GitHub/network access fails, the CLI gives <https://pandoc.org/installing.html>
and the `--pandoc <pandoc.exe>` alternative.

Explicit `--pandoc` takes precedence; otherwise the app searches its managed
download first, then absolute PATH entries. A missing or invalid managed pointer
falls through to PATH. Existing executable release numbers are not compared against
the preferred download version. The emitted `pandoc-api-version` and supported
AST/IR contract still have to match: a genuinely incompatible format is rejected
with its actual version/path, before writing IR or starting Hancom.

The repository wrapper `convert.ps1` keeps its development-default pinned path
for repeatable fixtures; `-PandocPath` can select another installed executable.
The Rust executable has no hard-coded checkout/tool installation path.

Pandoc is upstream software by John MacFarlane and contributors, licensed under
GPL-2.0-or-later with additional component notices in its COPYRIGHT. This design
downloads the unmodified official distribution directly to the user's machine,
preserves notices, and links the corresponding source; it does not vendor or
relicense Pandoc. See the [official installation guidance](https://pandoc.org/installing.html)
and [upstream copyright notice](https://github.com/jgm/pandoc/blob/main/COPYRIGHT).

## .NET and Rust

.NET SDK and Rust pins are for developers. Users need .NET 10 x64 runtime,
not either compiler. All stable 10.0.x patches are accepted; a different .NET
major is not automatically compatible with this target. Runtime presence is
checked separately from Hancom readiness. C# remains a framework-dependent
single EXE with no loose runtime JSON files.

## Verification

2026-09-24: native Rust setup tests replace the former mocked PowerShell setup
test. Tests cover managed-before-PATH selection, invalid pointers, preferred
release fallback, missing upstream digest, unsafe ZIP paths, and preservation
of current.txt on integrity, extraction, compatibility and copyright-download
failures. Temporary staging directories are removed on these failures. The
original ZIP and all extracted documentation remain in successful installations.
Downloads are bounded at 128 MiB per archive and 4 MiB per metadata/notice;
extraction allows at most 10,000 entries and 1 GiB total uncompressed content.
Symlinks, traversal paths and duplicate output files are rejected. If publishing
the pointer itself fails, the previous pointer survives and the complete new
directory may remain inactive for inspection.

Live verification downloaded official Pandoc 3.10.1 using the native Rust path
and selected the new managed installation for the CommonMark source-metadata
smoke test. CommonMark/direct-JSON equivalence, image rebasing and four rejection
cases passed without COM. All 14 application unit tests and the repository
contract smoke passed. The release Rust EXE is 5,521,920 bytes (previously
4,589,056 bytes); the framework-dependent C# worker is unchanged.

2026-09-23: official preferred Pandoc 3.10.1 downloaded successfully into the
documented user cache, preserving upstream ZIP/docs/COPYRIGHT/source links.
Offline mocked-network tests cover preferred-download failure with latest-stable
fallback and digest failure preserving the previous current.txt pointer. These
tests do not claim a future Pandoc release is compatible.

Rust application tests (11), C# declaration/security checks, contract smoke,
CommonMark fixture and source-metadata regressions passed. The canonical
open-only probe passed without a fixed module hash gate or registration changes.

`tools/smoke/test-standalone-hwp.ps1` is an explicit live test in the verified
Windows PowerShell 5.1 context. It copied two EXEs, the tagged HWP template,
manuscript and image into a fresh system temporary directory outside the
repository, used automatically discovered downloaded Pandoc, and produced
result.hwp without lock/schema/runtime JSON sidecars. Save/reopen and template
preservation passed. Four PNGs in `artifacts/standalone-dependencies-pages/`
match the earlier report baseline exactly. The .NET runtime and registered
Hancom security module remained existing workstation prerequisites.
