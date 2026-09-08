# AURI basic-research profile

`investigation-v0.1.json` is the closed profile consumed by the C# minimal-HWP
preview. It declares the exact tracked fixture identity, symbolic-to-native
paragraph styles, `바탕글` reset style, document-end investigation target,
minimal box and caption prototypes, list indentation/numbering, figure layout,
and required native capabilities.

`minimal-marker-investigation-v0.1.json` binds the repository-owned
`tests/fixtures/templates/minimal-marker.hwp`. It has the same native style and
prototype contract but requires one dedicated end marker followed by an empty
paragraph. This paired fixture proves marker preflight and safe whole-paragraph
removal separately from the full external template.

These two tracked HWP files are the primary comparison pair. `minimal.hwp` is
the untouched, deliberately small example supplied for this investigation;
`minimal-marker.hwp` is its working copy with only the guarded insertion target
added at the document end. Rendering starts from the marked copy, removes that
paragraph, and leaves the original example body in place above the generated
content. The full report is not the runtime basis for this path.

The native style definitions in `minimal.hwp` are authoritative for this
fixture and are locked indirectly by its exact file hash. A read-only inspection
confirmed that `본문`, headings 1 through 4, `표그림_캡션`, and
`출처 및 하단설명` have the expected names and native attributes. It also found
that this small file's `박스내용` is 8 pt KoPubWorld돋움체 Light with 160%
line spacing, while the separate basic-research style-list document describes
8 pt Medium with 145% line spacing. The investigation renderer follows the
actual small template and does not silently rewrite it from that external
reference table.

Both profiles have status `investigation`. The original
`investigation-v0.1.json` identifies
`tests/fixtures/templates/minimal.hwp`, not the full third-party AURI report
template. The actual reference HWP remains under local `reference/` input until
its provenance, redistribution policy, unique insertion target, and different
box/caption structures are resolved and verified in a production profile.

The read-only structure inventory on 2026-08-31 identified the local full
reference file as 3,219,968 bytes with SHA-256
`0BA84133775B182C76ACE779082ED77E4BCE0743C00733FB6C7AF7CD43B3BAF1`.
It contains 6 sections, 538 root paragraphs, 1,306 total paragraphs, 49 tables,
13 pictures, and 4 Figure automatic-number controls. No field- or
bookmark-named HWPML element was present, so this exact file does not establish
a named insertion target.

The representative full-template figure is one inline (`TreatAsChar=true`)
picture anchored in a root `바탕글` paragraph. Its bottom caption contains a
`표그림_캡션` paragraph with one native Figure `AUTONUM`, followed by two
`출처 및 하단설명` paragraphs in the inspected example. The representative box
is one inline table anchored in a root `바탕글` paragraph. Its table content has
one `박스제목` paragraph and four `박스내용` paragraphs; its bottom caption has
one `출처 및 하단설명` paragraph. These structures differ from both
fixture-specific prototype selectors in `investigation-v0.1.json`.

Consequently, neither the observed root indexes nor a matching sample-content
string may be promoted as the insertion contract. A production profile still
needs a deliberately unique named marker in a separately managed runtime
template, or a documented and tested structural replacement region. Until one
is adopted, the full reference document remains inspection-only.

For continuing visual comparison, use the tracked `minimal.hwp` fixture as
the basis for the user-authored template. The preview's `prepare-template-pair`
mode creates two ignored artifacts: `artifacts/minimal-baseline.hwp`, a
byte-identical copy, and `artifacts/minimal-marked.hwp`, a Hancom-saved copy
with one dedicated `{{MD2HWP_INSERTION_TARGET_V0_1}}` paragraph at the document
end. The full reference document is not an input to this workflow.

The original profile uses document-end insertion. The marker profile binds the
tracked `minimal-marker.hwp`, requiring the marker and an empty successor at
the document end. Other generated copies or user-authored templates require
their own verified identity. This remains an investigation insertion contract.
