# md2hwp provenance

This directory is copied without code changes from
https://github.com/edwardkim/rhwp/tree/1a76570e833917d15817415a53c09ad61ab3203f/vendor/svg2pdf.

It contains rhwp's patched svg2pdf 0.13.0. The upstream fork commit and patch
history are documented in RHWP_PATCHES.md. LICENSE-MIT, LICENSE-APACHE and NOTICE
are retained verbatim. The root Cargo patch is required because dependency
workspace patches do not propagate to this workspace.
