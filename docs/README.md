# Documentation map

Current repository documentation is divided by authority and audience:

- [`specifications/`](specifications/): normative, versioned contracts such as
  project IR;
- [`design/`](design/): current architecture and repository-wide conventions;
- [`decisions/`](decisions/): architecture decision records (ADRs);
- [`development/`](development/): contributor and reference-workstation
  procedures.

Add `guides/` when a production HWP/HWPX user or operator workflow exists. The
current CommonMark-to-IR development command is documented in the root and app
READMEs; it is not yet an end-user document converter.

The authoritative current structure is
[`design/repository-layout.md`](design/repository-layout.md). ADRs explain why
the structure changed but do not replace that current map.

Unfinished personal notes and investigation scratch files do not belong here.
Keep disposable material below ignored `tmp/notes/`, and keep durable private
notes outside the repository. Downloaded pages used as research inputs belong
under ignored `reference/`. Evidence used to justify a durable decision should
be summarized in a design document or ADR before relying on it.
