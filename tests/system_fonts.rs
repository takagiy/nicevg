//! Widths measured in this machine's fonts, which the other tests leave out
//! (`NICEVG_SYSTEM_FONTS=0`) so that every machine measures the same.

/// Given seven hiragana, seven katakana, and two kanji with a comma after them,
///   at 13px in generic families, which have none of them on most machines
/// When they are measured in this machine's fonts
/// Then each character is about a font size wide, as fonts for documents
///   draw them: the hiragana 91px and the katakana 90px or 91px, not 71px and
///   68px as in Yu Gothic UI, the font Windows falls back to, and the kanji
///   with the comma 39px
#[test]
fn measures_kana_and_han_as_wide_as_fonts_for_documents_draw_them() {
    // SAFETY: this is the only test in its binary, so no other thread reads
    // the environment.
    unsafe { std::env::remove_var("NICEVG_SYSTEM_FONTS") };

    for family in ["sans-serif", "serif", ""] {
        let width = |text| nicevg::text_width(text, 13.0, family, "normal");
        assert_eq!(width("あいうえおかき"), 91.0, "{family}");
        assert!((90.0..=91.0).contains(&width("アイウエオカキ")), "{family}");
        assert_eq!(width("日本、"), 39.0, "{family}");
    }
}
