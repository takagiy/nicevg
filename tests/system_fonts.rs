//! Widths measured with this machine's fonts, which the other tests leave out
//! (`NICEVG_SYSTEM_FONTS=0`) so that every machine measures the same.

/// Given seven hiragana, seven katakana, two kanji with a comma after them,
///   and four narrow Latin letters, at 13px in generic families
/// When they are measured with this machine's fonts
/// Then each is estimated from its characters, as on a machine without
///   fonts, not measured in the fonts this machine resolves the families
///   to: the kana 91px, not 71px and 68px as in Yu Gothic UI, the font
///   Windows falls back to, the kanji with the comma 39px, and the Latin
///   letters 17px, not 12px as in Arial
#[test]
fn estimates_text_in_generic_families_whatever_fonts_the_machine_has() {
    // SAFETY: this is the only test in its binary, so no other thread reads
    // the environment.
    unsafe { std::env::remove_var("NICEVG_SYSTEM_FONTS") };

    for family in ["sans-serif", "serif", ""] {
        let width = |text| nicevg::text_width(text, 13.0, family, "normal");
        assert_eq!(width("あいうえおかき"), 91.0, "{family}");
        assert_eq!(width("アイウエオカキ"), 91.0, "{family}");
        assert_eq!(width("日本、"), 39.0, "{family}");
        assert_eq!(width("illi"), 17.0, "{family}");
    }
}
