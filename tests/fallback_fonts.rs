//! Widths of scripts the declared fonts lack, measured in this machine's
//! fonts on Windows, which the other tests leave out (`NICEVG_SYSTEM_FONTS=0`).
#![cfg(windows)]

/// Given Thai and Georgian words at 13px in sans-serif, on Windows, whose Arial
///   has neither: Chrome 157 draws the Thai in Tahoma, 53.84px and 23.48px
///   wide, and the Georgian in Sylfaen, 60.58px wide
/// When they are measured in this machine's fonts
/// Then each is as wide as Chrome draws it, to the nearest pixel, not as
///   Leelawadee UI and Segoe UI draw them, the fonts Windows falls back to
#[test]
fn measures_thai_and_georgian_the_declared_fonts_lack_as_chrome_draws_them() {
    // SAFETY: this is the only test in its binary, so no other thread reads
    // the environment.
    unsafe { std::env::remove_var("NICEVG_SYSTEM_FONTS") };

    let width = |text| nicevg::text_width(text, 13.0, "sans-serif", "normal");

    for (text, chrome) in [("ภาษาไทย", 53.84), ("ไทย", 23.48), ("ქართული", 60.58)] {
        assert!(
            (width(text) - chrome).abs() <= 0.5,
            "{text}: {} for {chrome}",
            width(text)
        );
    }
}
