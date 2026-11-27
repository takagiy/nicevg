//! The width of a text: as this machine's fonts draw it in the families the
//! text names, and estimated from its characters where it names none of
//! them, or where they lack a character.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use icu_properties::props::{EmojiPresentation, Script};
use icu_properties::{CodePointMapData, CodePointSetData};
use parley::fontique::{self, Blob, Collection, CollectionOptions, GenericFamily, ScriptExt};
use parley::{FontContext, FontFamilyName, FontWeight, Layout, LayoutContext, StyleProperty};
use unicode_width::UnicodeWidthChar;

/// Liberation Sans, as wide as Arial: a font every machine measures a text in
/// that names it, those without the machine's fonts too.
const OWN_FONTS: [&[u8]; 2] = [
    include_bytes!("../fonts/LiberationSans-Regular.ttf"),
    include_bytes!("../fonts/LiberationSans-Bold.ttf"),
];

/// The em a character of each script takes, regular and bold, where no font
/// of the families a text names draws it, and `char_widths.tsv` has no width
/// for it: the least, to 0.01em, in which 95% of the script's words from
/// Wikipedia fit in each of the fonts browsers on Windows, macOS, Linux and
/// Android draw it in by default.
const SCRIPTS: [(Script, [f32; 2]); 24] = [
    // Halfwidth forms, the only katakana and Hangul one column wide.
    (Script::Katakana, [0.5, 0.5]),
    (Script::Hangul, [0.5, 0.5]),
    (Script::Latin, [0.76, 0.82]),
    (Script::Cyrillic, [0.70, 0.78]),
    (Script::Greek, [0.73, 0.76]),
    (Script::Arabic, [0.71, 0.82]),
    (Script::Hebrew, [0.69, 0.76]),
    (Script::Armenian, [0.81, 0.89]),
    (Script::Georgian, [0.74, 0.81]),
    (Script::Thai, [0.66, 0.72]),
    (Script::Lao, [0.71, 0.84]),
    (Script::Khmer, [0.85, 0.86]),
    (Script::Myanmar, [1.13, 1.11]),
    (Script::Tibetan, [0.71, 0.75]),
    (Script::Ethiopic, [0.83, 0.91]),
    (Script::Devanagari, [0.74, 0.79]),
    (Script::Bengali, [0.84, 0.93]),
    (Script::Gurmukhi, [0.75, 0.81]),
    (Script::Gujarati, [0.77, 0.81]),
    (Script::Tamil, [1.17, 1.24]),
    (Script::Telugu, [1.02, 1.11]),
    (Script::Kannada, [1.11, 1.20]),
    (Script::Malayalam, [1.30, 1.45]),
    (Script::Sinhala, [1.03, 1.16]),
];

/// A character Unicode gives two columns (East Asian Width), such as kana,
/// Han and Hangul: a font size wide in every font for documents measured.
const WIDE: [f32; 2] = [1.0, 1.0];

/// A character of a script measured in no font, as wide as most characters.
const OTHER: [f32; 2] = [1.0, 1.0];

/// An emoji: the widest of Segoe UI Emoji and Noto Color Emoji.
const EMOJI: [f32; 2] = [1.38, 1.38];

/// Any Latin, Cyrillic or Greek letter, digit, space or punctuation in a
/// monospace family: the widest of the monospace fonts measured.
const MONOSPACE: [f32; 2] = [0.61, 0.61];

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
        for font in OWN_FONTS {
            collection.register_fonts(Blob::from(font.to_vec()), None);
        }
        // A character the named families lack is estimated, not measured in
        // a font this machine falls back to, which a reader's may not be.
        let scripts = fontique::Script::all_samples().iter().map(|(script, _)| *script);
        let rest = [*b"Zyyy", *b"Zinh", *b"Zzzz"].map(fontique::Script::from_bytes);
        for script in scripts.chain(rest) {
            collection.set_fallbacks(script, std::iter::empty());
        }
        collection.set_generic_families(GenericFamily::Emoji, std::iter::empty());
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
/// and `font-weight` it declares. The families it names before its first
/// generic one that this machine has are measured as they draw it. The rest,
/// all of a text that names none, or only a generic family, and the
/// characters those families lack, are estimated character by character: no
/// font is known to draw them on a reader's machine.
pub fn text_width(text: &str, font_size: f64, font_family: &str, font_weight: &str) -> f64 {
    let weight = FontWeight::parse_css(font_weight).unwrap_or_default();
    let bold = weight.value() >= 600.0;
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
    let size = font_size as f32;
    let (named, monospace) = named_families(&mut fonts.collection, font_family);
    let width = if named.is_empty() {
        estimate(text, monospace, bold) * size
    } else {
        let mut builder = layouts.ranged_builder(fonts, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(named.as_slice().into()));
        builder.push_default(StyleProperty::FontSize(size));
        builder.push_default(StyleProperty::FontWeight(weight));
        let mut layout: Layout<()> = builder.build(text);
        layout.break_all_lines(None);
        // A character none of the families has is laid out as a glyph for a
        // missing one, which is no measure of what a reader's font draws.
        let unseen: f32 = layout
            .lines()
            .flat_map(|line| line.runs())
            .flat_map(|run| run.clusters())
            .filter(|cluster| cluster.glyphs().any(|glyph| glyph.id == 0))
            .map(|cluster| estimate(&text[cluster.text_range()], monospace, bold) * size - cluster.advance())
            .sum();
        layout.width() + unseen
    };
    let width = f64::from(width).round();
    known
        .entry(font_family.to_owned())
        .or_default()
        .insert(text.to_owned(), width);
    *kept += 1;
    width
}

/// The families a `font-family` list names before its first generic family
/// that this machine has, and whether that generic family is monospace. A
/// browser draws in those families what they have, and the rest in a generic
/// family, `serif` where the list ends without one.
fn named_families<'a>(collection: &mut Collection, list: &'a str) -> (Vec<FontFamilyName<'a>>, bool) {
    let mut named = vec![];
    for family in FontFamilyName::parse_css_list(list).map_while(Result::ok) {
        match family {
            FontFamilyName::Generic(generic) => {
                let monospace = matches!(generic, GenericFamily::Monospace | GenericFamily::UiMonospace);
                return (named, monospace);
            }
            FontFamilyName::Named(ref name) if collection.family_id(name).is_some() => named.push(family),
            FontFamilyName::Named(_) => {}
        }
    }
    (named, false)
}

/// How many em a text takes, regular or bold, where no font is known to draw
/// it: a Latin, Greek or Cyrillic character, punctuation or symbol as
/// `char_widths.tsv` gives it, any other by its script (`SCRIPTS`), with
/// combining ones none, those Unicode gives two columns a font size, and an
/// emoji, or a character an emoji variation selector follows, as an emoji.
fn estimate(text: &str, monospace: bool, bold: bool) -> f32 {
    let scripts = CodePointMapData::<Script>::new();
    let emoji = CodePointSetData::new::<EmojiPresentation>();
    let mut characters = text.chars().peekable();
    let mut ems = 0.0;
    while let Some(character) = characters.next() {
        let script = scripts.get(character);
        let common = matches!(script, Script::Common | Script::Inherited);
        let narrow = character.width() == Some(1);
        // Punctuation and symbols East Asian texts set two columns wide, such
        // as ① and ※, are drawn so where Latin fonts lack them.
        let columns = if common {
            character.width_cjk()
        } else {
            character.width()
        };
        let em = if emoji.contains(character) || characters.peek() == Some(&'\u{FE0F}') {
            EMOJI
        } else if matches!(columns, Some(0) | None) {
            [0.0; 2]
        } else if monospace && narrow && (common || matches!(script, Script::Latin | Script::Cyrillic | Script::Greek))
        {
            MONOSPACE
        } else if let Some(em) = char_widths().get(&character) {
            *em
        } else if columns == Some(2) {
            WIDE
        } else {
            let script = if common { Script::Latin } else { script };
            SCRIPTS
                .iter()
                .find(|(known, _)| *known == script)
                .map_or(OTHER, |(_, em)| *em)
        };
        ems += em[usize::from(bold)];
    }
    ems
}

/// The em of each Latin, Greek and Cyrillic character, punctuation and
/// symbol `char_widths.tsv` lists, regular and bold: the widest of the fonts
/// browsers on Windows, macOS, Linux and Android draw them in by default.
fn char_widths() -> &'static HashMap<char, [f32; 2]> {
    static WIDTHS: OnceLock<HashMap<char, [f32; 2]>> = OnceLock::new();
    WIDTHS.get_or_init(|| {
        include_str!("char_widths.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let character = char::from_u32(u32::from_str_radix(fields.next()?, 16).ok()?)?;
                Some((character, [fields.next()?.parse().ok()?, fields.next()?.parse().ok()?]))
            })
            .collect()
    })
}
