use md2hwp_math::hml_to_tex;

#[test]
fn color_can_include_the_closing_delimiter() {
    assert_eq!(
        hml_to_tex("LEFT ( black {color {255,0,0} {red RIGHT )}}").unwrap(),
        r"{\left(black \color[RGB]{255,0,0}red\right)}"
    );
}

#[test]
fn regular_color_and_nested_delimiters_are_preserved() {
    assert_eq!(
        hml_to_tex("LEFT ( color {255,0,0} {LEFT [ x RIGHT ]} RIGHT )").unwrap(),
        r"\left(\textcolor[RGB]{255,0,0}{\left[x\right]}\right)"
    );
    assert!(hml_to_tex("color {255,0,0} {x RIGHT )}").is_err());
}
