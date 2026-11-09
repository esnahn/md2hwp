// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn verb_is_explicitly_unsupported() {
    for tex in [r"\verb|a  b|", r"\verb*|a  b|", r"\verb \verb "] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
