# Pandoc Unicode normalization observation

Status: observed on the pinned reference parser. The separate md2hwp NFC
contract is defined in `docs/specifications/ir-v0.1.md`.

## Scope

This investigation used the official Pandoc 3.10.1 Windows x86_64 binary
pinned in `dependencies/lock.json`. The input contained adjacent scalar values
`U+1100 HANGUL CHOSEONG KIYEOK` and `U+1161 HANGUL JUNGSEONG A`.

The source file was checked before invocation and contained those two scalar
values rather than `U+AC00 HANGUL SYLLABLE GA`.

## Observation

The following readers produced different Pandoc JSON `Str` values:

| Reader | Pandoc JSON scalar sequence |
| --- | --- |
| `commonmark` | `U+AC00` |
| `commonmark_x` | `U+AC00` |
| `gfm` | `U+AC00` |
| `markdown_strict` | `U+1100 U+1161` |
| `markdown` | `U+1100 U+1161` |
| `rst` | `U+1100 U+1161` |

Using numeric CommonMark character references (`&#x1100;&#x1161;`) also
produced `U+1100 U+1161`. md2hwp does not rewrite source notation before
Pandoc, but source-to-IR normalization subsequently converts human-readable
content to NFC.

The CommonMark compatibility fixture intentionally uses the literal decomposed
sequence. Its generated IR therefore contains the composed value observed at
the Pandoc boundary. This makes the behavior visible in byte/code-point tests
even though canonical IR would also compose a decomposed AST value.

## Boundary conclusion

`apps/md2hwp` sends UTF-8 source to Pandoc without normalization.
`md2hwp-core` converts human-readable text, titles, and verbatim content from
Pandoc JSON to NFC before constructing IR. Opaque link targets and image paths
are excluded: their scalar values are preserved exactly, including NFD, so
normalization cannot redirect a URL or select another file. Direct IR input
containing non-NFC human-readable content is invalid. md2hwp does not apply
NFD, NFKC, or NFKD.

Composition observed before the core boundary remains a property of the
selected Pandoc reader/version. The IR result no longer depends on that
property because NFC is an explicit project invariant.

Changing Pandoc versions or readers requires rerunning this probe to keep the
boundary observation accurate; the explicit IR NFC invariant remains stable.
