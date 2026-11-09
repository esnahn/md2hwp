// SPDX-License-Identifier: AGPL-3.0-only
//! Color specifications accepted by the TeX math adapter.

/// Resolve KaTeX/CSS named colors and explicit models to native 8-bit RGB.
pub(crate) fn parse_color(model: &str, value: &str) -> Result<[u8; 3], String> {
    let value = value.trim();
    match model.trim() {
        "" => {
            if let Some(hex) = value.strip_prefix('#') {
                return parse_hex(hex);
            }
            CSS_NAMED_COLORS
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(value))
                .map(|(_, rgb)| *rgb)
                .ok_or_else(|| format!("unsupported color name: {value}"))
        }
        "RGB" => {
            let values: Vec<_> = value.split(',').collect();
            if values.len() != 3 {
                return Err("RGB requires three components".into());
            }
            let mut rgb = [0; 3];
            for (out, component) in rgb.iter_mut().zip(values) {
                *out = component
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| "RGB components must be integers 0..255".to_string())?;
            }
            Ok(rgb)
        }
        "HTML" => parse_hex(value.strip_prefix('#').unwrap_or(value)),
        _ => Err(format!("unsupported color model: {model}")),
    }
}

// CSS named RGB colors, including spelling aliases.
const CSS_NAMED_COLORS: [(&str, [u8; 3]); 148] = [
    ("aliceblue", [240, 248, 255]),
    ("antiquewhite", [250, 235, 215]),
    ("aqua", [0, 255, 255]),
    ("aquamarine", [127, 255, 212]),
    ("azure", [240, 255, 255]),
    ("beige", [245, 245, 220]),
    ("bisque", [255, 228, 196]),
    ("black", [0, 0, 0]),
    ("blanchedalmond", [255, 235, 205]),
    ("blue", [0, 0, 255]),
    ("blueviolet", [138, 43, 226]),
    ("brown", [165, 42, 42]),
    ("burlywood", [222, 184, 135]),
    ("cadetblue", [95, 158, 160]),
    ("chartreuse", [127, 255, 0]),
    ("chocolate", [210, 105, 30]),
    ("coral", [255, 127, 80]),
    ("cornflowerblue", [100, 149, 237]),
    ("cornsilk", [255, 248, 220]),
    ("crimson", [220, 20, 60]),
    ("cyan", [0, 255, 255]),
    ("darkblue", [0, 0, 139]),
    ("darkcyan", [0, 139, 139]),
    ("darkgoldenrod", [184, 134, 11]),
    ("darkgray", [169, 169, 169]),
    ("darkgreen", [0, 100, 0]),
    ("darkgrey", [169, 169, 169]),
    ("darkkhaki", [189, 183, 107]),
    ("darkmagenta", [139, 0, 139]),
    ("darkolivegreen", [85, 107, 47]),
    ("darkorange", [255, 140, 0]),
    ("darkorchid", [153, 50, 204]),
    ("darkred", [139, 0, 0]),
    ("darksalmon", [233, 150, 122]),
    ("darkseagreen", [143, 188, 143]),
    ("darkslateblue", [72, 61, 139]),
    ("darkslategray", [47, 79, 79]),
    ("darkslategrey", [47, 79, 79]),
    ("darkturquoise", [0, 206, 209]),
    ("darkviolet", [148, 0, 211]),
    ("deeppink", [255, 20, 147]),
    ("deepskyblue", [0, 191, 255]),
    ("dimgray", [105, 105, 105]),
    ("dimgrey", [105, 105, 105]),
    ("dodgerblue", [30, 144, 255]),
    ("firebrick", [178, 34, 34]),
    ("floralwhite", [255, 250, 240]),
    ("forestgreen", [34, 139, 34]),
    ("fuchsia", [255, 0, 255]),
    ("gainsboro", [220, 220, 220]),
    ("ghostwhite", [248, 248, 255]),
    ("gold", [255, 215, 0]),
    ("goldenrod", [218, 165, 32]),
    ("gray", [128, 128, 128]),
    ("green", [0, 128, 0]),
    ("greenyellow", [173, 255, 47]),
    ("grey", [128, 128, 128]),
    ("honeydew", [240, 255, 240]),
    ("hotpink", [255, 105, 180]),
    ("indianred", [205, 92, 92]),
    ("indigo", [75, 0, 130]),
    ("ivory", [255, 255, 240]),
    ("khaki", [240, 230, 140]),
    ("lavender", [230, 230, 250]),
    ("lavenderblush", [255, 240, 245]),
    ("lawngreen", [124, 252, 0]),
    ("lemonchiffon", [255, 250, 205]),
    ("lightblue", [173, 216, 230]),
    ("lightcoral", [240, 128, 128]),
    ("lightcyan", [224, 255, 255]),
    ("lightgoldenrodyellow", [250, 250, 210]),
    ("lightgray", [211, 211, 211]),
    ("lightgreen", [144, 238, 144]),
    ("lightgrey", [211, 211, 211]),
    ("lightpink", [255, 182, 193]),
    ("lightsalmon", [255, 160, 122]),
    ("lightseagreen", [32, 178, 170]),
    ("lightskyblue", [135, 206, 250]),
    ("lightslategray", [119, 136, 153]),
    ("lightslategrey", [119, 136, 153]),
    ("lightsteelblue", [176, 196, 222]),
    ("lightyellow", [255, 255, 224]),
    ("lime", [0, 255, 0]),
    ("limegreen", [50, 205, 50]),
    ("linen", [250, 240, 230]),
    ("magenta", [255, 0, 255]),
    ("maroon", [128, 0, 0]),
    ("mediumaquamarine", [102, 205, 170]),
    ("mediumblue", [0, 0, 205]),
    ("mediumorchid", [186, 85, 211]),
    ("mediumpurple", [147, 112, 219]),
    ("mediumseagreen", [60, 179, 113]),
    ("mediumslateblue", [123, 104, 238]),
    ("mediumspringgreen", [0, 250, 154]),
    ("mediumturquoise", [72, 209, 204]),
    ("mediumvioletred", [199, 21, 133]),
    ("midnightblue", [25, 25, 112]),
    ("mintcream", [245, 255, 250]),
    ("mistyrose", [255, 228, 225]),
    ("moccasin", [255, 228, 181]),
    ("navajowhite", [255, 222, 173]),
    ("navy", [0, 0, 128]),
    ("oldlace", [253, 245, 230]),
    ("olive", [128, 128, 0]),
    ("olivedrab", [107, 142, 35]),
    ("orange", [255, 165, 0]),
    ("orangered", [255, 69, 0]),
    ("orchid", [218, 112, 214]),
    ("palegoldenrod", [238, 232, 170]),
    ("palegreen", [152, 251, 152]),
    ("paleturquoise", [175, 238, 238]),
    ("palevioletred", [219, 112, 147]),
    ("papayawhip", [255, 239, 213]),
    ("peachpuff", [255, 218, 185]),
    ("peru", [205, 133, 63]),
    ("pink", [255, 192, 203]),
    ("plum", [221, 160, 221]),
    ("powderblue", [176, 224, 230]),
    ("purple", [128, 0, 128]),
    ("rebeccapurple", [102, 51, 153]),
    ("red", [255, 0, 0]),
    ("rosybrown", [188, 143, 143]),
    ("royalblue", [65, 105, 225]),
    ("saddlebrown", [139, 69, 19]),
    ("salmon", [250, 128, 114]),
    ("sandybrown", [244, 164, 96]),
    ("seagreen", [46, 139, 87]),
    ("seashell", [255, 245, 238]),
    ("sienna", [160, 82, 45]),
    ("silver", [192, 192, 192]),
    ("skyblue", [135, 206, 235]),
    ("slateblue", [106, 90, 205]),
    ("slategray", [112, 128, 144]),
    ("slategrey", [112, 128, 144]),
    ("snow", [255, 250, 250]),
    ("springgreen", [0, 255, 127]),
    ("steelblue", [70, 130, 180]),
    ("tan", [210, 180, 140]),
    ("teal", [0, 128, 128]),
    ("thistle", [216, 191, 216]),
    ("tomato", [255, 99, 71]),
    ("turquoise", [64, 224, 208]),
    ("violet", [238, 130, 238]),
    ("wheat", [245, 222, 179]),
    ("white", [255, 255, 255]),
    ("whitesmoke", [245, 245, 245]),
    ("yellow", [255, 255, 0]),
    ("yellowgreen", [154, 205, 50]),
];

fn parse_hex(value: &str) -> Result<[u8; 3], String> {
    let digits = value.as_bytes();
    let digit = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    };
    let invalid = || "color must contain three or six hexadecimal digits".to_string();
    match digits {
        [r, g, b] => Ok([
            digit(*r).ok_or_else(invalid)? * 17,
            digit(*g).ok_or_else(invalid)? * 17,
            digit(*b).ok_or_else(invalid)? * 17,
        ]),
        [r1, r2, g1, g2, b1, b2] => Ok([
            digit(*r1).ok_or_else(invalid)? * 16 + digit(*r2).ok_or_else(invalid)?,
            digit(*g1).ok_or_else(invalid)? * 16 + digit(*g2).ok_or_else(invalid)?,
            digit(*b1).ok_or_else(invalid)? * 16 + digit(*b2).ok_or_else(invalid)?,
        ]),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_color;

    #[test]
    fn named_colors_are_ascii_case_insensitive_and_include_aliases() {
        assert_eq!(parse_color("", "Blue"), Ok([0, 0, 255]));
        assert_eq!(parse_color("", "ReBeCcApUrPlE"), Ok([102, 51, 153]));
        for (a, b, expected) in [
            ("aqua", "cyan", [0, 255, 255]),
            ("fuchsia", "magenta", [255, 0, 255]),
            ("gray", "grey", [128, 128, 128]),
            ("darkslategray", "darkslategrey", [47, 79, 79]),
        ] {
            assert_eq!(parse_color("", a), Ok(expected));
            assert_eq!(parse_color("", b), Ok(expected));
        }
        for value in ["unknown", "transparent", "currentColor"] {
            assert!(parse_color("", value).is_err(), "{value}");
        }
    }

    #[test]
    fn parses_hex_and_rgb_models() {
        assert_eq!(parse_color("", "#aB3"), Ok([170, 187, 51]));
        assert_eq!(parse_color("", "#012aFF"), Ok([1, 42, 255]));
        assert_eq!(parse_color("HTML", "Ff8000"), Ok([255, 128, 0]));
        assert_eq!(parse_color("RGB", " 1, 42, 255 "), Ok([1, 42, 255]));
    }

    #[test]
    fn rejects_invalid_specs() {
        for (model, value) in [
            ("", "#12"),
            ("", "#12345g"),
            ("HTML", "ééé"),
            ("RGB", "1,2"),
            ("RGB", "1,2,256"),
            ("rgb", "1,2,3"),
        ] {
            assert!(parse_color(model, value).is_err(), "{model}: {value}");
        }
    }
}
