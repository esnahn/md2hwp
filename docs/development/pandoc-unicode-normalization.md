# Pandoc Unicode normalization observation

Status: observed on the pinned reference parser; not an md2hwp normalization
policy.

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
produced `U+1100 U+1161`, but md2hwp does not require or automatically rewrite
source text into that spelling.

The CommonMark compatibility fixture intentionally uses the literal decomposed
sequence. Its generated IR therefore contains the composed value observed at
the Pandoc boundary. This makes the behavior visible in byte/code-point tests
even though the two strings appear equivalent in ordinary visual inspection.

## Boundary conclusion

`apps/md2hwp` sends UTF-8 source to Pandoc without normalization.
`md2hwp-core` does not apply NFC, NFD, NFKC, or NFKD and preserves the scalar
values present in Pandoc JSON. Composition observed before that boundary is a
property of the selected Pandoc reader/version, not a transformation requested
or implemented by md2hwp.

Changing Pandoc versions or readers requires rerunning this probe because the
observed behavior is not generalized into a compatibility guarantee.
