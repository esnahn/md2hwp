# Packaging

Store versioned package manifests, installer definitions, and release assembly
scripts here after a distribution format is selected. Generated archives,
installers, checksums, and staging trees use the selected ecosystem or installer
tool's ignored output convention; no common release staging path is selected
yet.

Packaging must consume tracked source and pinned documented inputs. It must not
silently bundle `reference/`, `artifacts/`, `tmp/`, or the ignored
`dependencies/FilePathCheckerModuleExample.dll`.
