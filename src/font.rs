//! The width of a text as the fonts of this machine draw it.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use parley::fontique::{Blob, Collection, CollectionOptions, GenericFamily, Script};
use parley::{FontContext, FontWeight, Layout, LayoutContext, StyleProperty};
use unicode_width::UnicodeWidthStr;

/// Liberation Sans, as wide as Arial: the fonts a text is measured in where
/// the machine has none for it.
const OWN_FONTS: [&[u8]; 2] = [
    include_bytes!("../fonts/LiberationSans-Regular.ttf"),
    include_bytes!("../fonts/LiberationSans-Bold.ttf"),
];

/// Kana and Han, with the punctuation among them. Fonts for documents draw
/// each of their characters as wide as its columns say, while the font a
/// machine falls back to where the declared fonts lack them may not: the one
/// Windows picks has kana about a quarter narrower. Which font a reader's
/// browser picks is not known here, so their columns give their width.
const KANA_AND_HAN: [[u8; 4]; 3] = [*b"Hira", *b"Kana", *b"Hani"];

/// The fonts Chrome on Windows draws Thai and Georgian in where the declared
/// fonts lack them, the first of Chromium's lists for them there. Windows
/// itself falls back to Leelawadee UI, which draws Thai up to 16% narrower,
/// and to Segoe UI, which draws Georgian about a tenth wider. Chrome draws
/// the other scripts in the fonts Windows falls back to.
#[cfg(windows)]
const CHROME_FALLBACKS: [([u8; 4], &str); 2] = [(*b"Thai", "Tahoma"), (*b"Geor", "Sylfaen")];

/// How many widths are kept: past it all are dropped, so that a process
/// measuring ever new texts does not grow with them.
const KEPT_WIDTHS: usize = 100_000;

/// The fonts, and the widths measured so far by size and weight, family as
/// declared, then text, with how many they are.
struct Fonts {
    fonts: FontContext,
    layouts: LayoutContext<()>,
    measured: HashMap<(u64, u32), HashMap<String, HashMap<String, f64>>>,
    kept: usize,
}

fn fonts() -> &'static Mutex<Fonts> {
    static FONTS: OnceLock<Mutex<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| {
        // Without the machine's fonts, when `NICEVG_SYSTEM_FONTS` is 0, every
        // machine measures the same widths.
        let mut collection = Collection::new(CollectionOptions {
            system_fonts: std::env::var_os("NICEVG_SYSTEM_FONTS").is_none_or(|value| value != "0"),
            shared: false,
        });
        let own: Vec<_> = OWN_FONTS
            .iter()
            .flat_map(|font| collection.register_fonts(Blob::from(font.to_vec()), None))
            .map(|(family, _)| family)
            .collect();
        for generic in GenericFamily::all() {
            if collection.generic_families(*generic).next().is_none() {
                collection.set_generic_families(*generic, own.iter().copied());
            }
        }
        for script in KANA_AND_HAN.map(Script::from_bytes) {
            collection.set_fallbacks(script, std::iter::empty());
        }
        #[cfg(windows)]
        for (script, family) in CHROME_FALLBACKS {
            let script = Script::from_bytes(script);
            let system: Vec<_> = collection.fallback_families(script).collect();
            let chrome = collection.family_id(family);
            collection.set_fallbacks(script, chrome.into_iter().chain(system));
        }
        Mutex::new(Fonts {
            fonts: FontContext {
                collection,
                source_cache: Default::default(),
            },
            layouts: LayoutContext::new(),
            measured: HashMap::new(),
            kept: 0,
        })
    })
}

/// How wide a line of text is drawn at a font size, with the `font-family`
/// and `font-weight` it declares. A text of no family, or of families there
/// is no font of, is drawn in a serif font, as browsers draw it. A
/// character no font of this machine has, or kana or Han the declared fonts
/// lack, is taken as wide as the columns Unicode gives it (East Asian Width,
/// ambiguous ones as wide), half a font size each: a whole font size for a
/// wide character, none for a combining one.
pub fn text_width(text: &str, font_size: f64, font_family: &str, font_weight: &str) -> f64 {
    let weight = FontWeight::parse_css(font_weight).unwrap_or_default();
    let mut fonts = fonts().lock().expect("no measuring panics");
    let Fonts {
        fonts,
        layouts,
        measured,
        kept,
    } = &mut *fonts;
    if *kept >= KEPT_WIDTHS {
        measured.clear();
        *kept = 0;
    }
    let known = measured
        .entry((font_size.to_bits(), weight.value().to_bits()))
        .or_default();
    if let Some(width) = known.get(font_family).and_then(|texts| texts.get(text)) {
        return *width;
    }
    let families = [font_family, "serif"]
        .map(str::trim)
        .iter()
        .filter(|family| !family.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    let mut builder = layouts.ranged_builder(fonts, text, 1.0, false);
    builder.push_default(StyleProperty::FontFamily(families.as_str().into()));
    builder.push_default(StyleProperty::FontSize(font_size as f32));
    builder.push_default(StyleProperty::FontWeight(weight));
    let mut layout: Layout<()> = builder.build(text);
    layout.break_all_lines(None);
    // Such a character is laid out as its font's glyph for a missing one,
    // which is no measure of what a reader's font draws: its columns stand
    // in for that advance.
    let unseen: f32 = layout
        .lines()
        .flat_map(|line| line.runs())
        .flat_map(|run| run.clusters())
        .filter(|cluster| cluster.glyphs().any(|glyph| glyph.id == 0))
        .map(|cluster| text[cluster.text_range()].width_cjk() as f32 * font_size as f32 / 2.0 - cluster.advance())
        .sum();
    let width = f64::from(layout.width() + unseen).round();
    known
        .entry(font_family.to_owned())
        .or_default()
        .insert(text.to_owned(), width);
    *kept += 1;
    width
}
