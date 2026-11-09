// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn automatic_break_hints_do_not_create_visible_content_or_hard_breaks() {
    for command in ["nobreak", "allowbreak"] {
        let native = tex_to_hml(&format!(r"a+\{command} b=c")).unwrap();
        assert_eq!(native, "a + b = c");
        assert!(!native.contains('#'));
        assert_eq!(
            tex_to_hml(&format!(r"\frac{{a\{command}+b}}{{c}}")).unwrap(),
            "{{a + b} over {c}}"
        );
        assert!(tex_to_hml(&format!(r"\{command}bad x")).is_err());
    }
    assert!(tex_to_hml(r"a\\b").unwrap().contains("pile"));
}
