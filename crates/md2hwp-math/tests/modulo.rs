// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn modulo_commands_use_native_mod_and_parse_arguments() {
    assert_eq!(tex_to_hml(r"a\bmod b").unwrap(), "a `` mod `` b");
    assert_eq!(tex_to_hml(r"a\mod b").unwrap(), "a ~` mod ``` b");
    assert_eq!(tex_to_hml(r"\pmod{n+1}").unwrap(), "~ {( mod ``` n + 1 )}");
    assert_eq!(tex_to_hml(r"\pod{x}").unwrap(), "~ {( x )}");
    assert_eq!(tex_to_hml(r"\pmod12").unwrap(), "~ {( mod ``` 1 )} 2");
    assert_eq!(
        tex_to_hml(r"\displaystyle a\mod b").unwrap(),
        "a ~~ mod ``` b"
    );
    assert_eq!(tex_to_hml(r"\displaystyle\pod{x}").unwrap(), "~~ {( x )}");
    assert!(tex_to_hml(r"\pmod{\frac{1}{2}}").is_ok());
    for input in [r"\pmod", r"\pod{}", r"\pmod{ }", r"\pod{\unknown}"] {
        assert!(tex_to_hml(input).is_err(), "{input}");
    }
}
