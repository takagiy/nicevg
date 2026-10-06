//! Text as it is laid out: presentation properties read the way CSS
//! cascades them, lines split at positioned `tspan`s, and labels that
//! gather one or more texts.

use std::sync::LazyLock;

use regex::Regex;

use crate::diagram::{non_empty, number_attribute, translation};
use crate::geometry::{Bounds, enclosing, format_number, measure_text, parse_number, point, round, text_width};
use crate::xml::{Document, Element, Node, Path};

static LENGTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*([-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?)\s*([a-zA-Z%]*)\s*$").expect("valid pattern")
});

/// The declared value of a presentation property on one element: an inline
/// `style` declaration wins over the attribute of the same name.
pub fn own_property<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    style_declaration(element.attr("style"), name).or_else(|| non_empty(element.attr(name).trim()))
}

/// The last declaration of `name` in an inline style.
fn style_declaration<'a>(style: &'a str, name: &str) -> Option<&'a str> {
    style
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .filter(|(property, _)| property.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim().trim_end_matches("!important").trim())
        .rfind(|value| !value.is_empty())
}

/// Sets a presentation property where it takes effect: in the inline style
/// when the style declares it, otherwise as an attribute.
pub fn with_property(element: Element, name: &str, value: &str) -> Element {
    let style = element.attr("style");
    if style_declaration(style, name).is_none() {
        return element.with_attr(name, value);
    }
    let rewritten: Vec<String> = style
        .split(';')
        .map(|declaration| match declaration.split_once(':') {
            Some((property, _)) if property.trim().eq_ignore_ascii_case(name) => format!("{property}: {value}"),
            _ => declaration.to_owned(),
        })
        .collect();
    let rewritten = rewritten.join(";");
    element.with_attr("style", &rewritten)
}

/// The value of an inherited property: the nearest declaration on the
/// element or its ancestors.
pub fn inherited_property<'a>(document: &'a Document, path: &[usize], name: &str) -> Option<&'a str> {
    document
        .lineage(path)
        .into_iter()
        .filter_map(|element| own_property(element, name))
        .find(|value| *value != "inherit")
}

/// The font size in user units, resolving absolute units and sizes relative
/// to the parent's from the root down; browsers start at 16px.
pub fn font_size(document: &Document, path: &[usize]) -> f64 {
    document.lineage(path).into_iter().rev().fold(16.0, |parent, element| {
        own_property(element, "font-size")
            .and_then(|value| css_length(value, parent))
            .filter(|size| *size > 0.0)
            .unwrap_or(parent)
    })
}

/// How far below the top of a line's box its `y` lies: the alphabetic
/// baseline sits one font size down in a box 1.2 font sizes tall, a central
/// one in the middle, a hanging one at the top and a bottom one at the
/// bottom.
pub fn ascent(document: &Document, path: &[usize], font_size: f64) -> f64 {
    match inherited_property(document, path, "dominant-baseline").unwrap_or("auto") {
        "central" | "middle" => font_size * 0.6,
        "hanging" | "text-before-edge" | "text-top" => 0.0,
        "text-after-edge" | "text-bottom" | "ideographic" => font_size * 1.2,
        _ => font_size,
    }
}

/// A CSS length in user units; `em` and percentages are relative to
/// `reference`. Keywords and unknown units give `None`.
pub fn css_length(value: &str, reference: f64) -> Option<f64> {
    let captures = LENGTH.captures(value)?;
    let number = parse_number(&captures[1]);
    let scale = match captures[2].to_ascii_lowercase().as_str() {
        "" | "px" => 1.0,
        "pt" => 4.0 / 3.0,
        "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "em" => reference,
        "rem" => 16.0,
        "%" => reference / 100.0,
        _ => return None,
    };
    Some(number * scale)
}

/// One line of a text: the element whose `x` starts it, and its box.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub path: Path,
    pub bounds: Bounds,
}

/// A line being gathered: where it starts and its runs of text with their
/// font sizes.
struct Gathering {
    path: Path,
    x: f64,
    y: f64,
    runs: Vec<(String, f64)>,
}

/// The lines of a `text` element. The text starts the first line and each
/// `tspan` with its own `x` starts another, at its `y` or `dy` below the
/// line before. White space collapses as browsers render it, and lines
/// left empty take no space unless the whole text is empty.
pub fn lines(document: &Document, text: &[usize]) -> Vec<Line> {
    let element = document.element(text);
    let size = font_size(document, text);
    let first = Gathering {
        path: text.to_vec(),
        x: number_attribute(element, "x"),
        y: number_attribute(element, "y"),
        runs: Vec::new(),
    };
    let gathered = element
        .children
        .iter()
        .enumerate()
        .fold(vec![first], |mut gathered, (index, child)| {
            match child {
                Node::Text(content) | Node::CData(content) => {
                    gathered
                        .last_mut()
                        .expect("a first line")
                        .runs
                        .push((content.clone(), size));
                }
                Node::Element(child) => {
                    let path = [text, &[index]].concat();
                    let child_size = font_size(document, &path);
                    if child.name == "tspan" && child.has_attr("x") {
                        let previous = gathered.last().expect("a first line").y;
                        let y = if child.has_attr("y") {
                            number_attribute(child, "y")
                        } else {
                            previous + css_length(child.attr("dy"), child_size).unwrap_or(0.0)
                        };
                        gathered.push(Gathering {
                            path: path.clone(),
                            x: number_attribute(child, "x"),
                            y,
                            runs: Vec::new(),
                        });
                    }
                    gathered
                        .last_mut()
                        .expect("a first line")
                        .runs
                        .push((child.text_content(), child_size));
                }
                _ => {}
            }
            gathered
        });
    let measured: Vec<Line> = gathered
        .iter()
        .filter_map(|line| {
            let runs = collapse_white_space(&line.runs);
            (!runs.is_empty()).then(|| measure_line(document, line, &runs))
        })
        .collect();
    if measured.is_empty() {
        vec![measure_line(document, &gathered[0], &[])]
    } else {
        measured
    }
}

/// Runs of a line with white space collapsed: every sequence of spaces,
/// tabs and line breaks becomes one space, and the line's ends are trimmed.
fn collapse_white_space(runs: &[(String, f64)]) -> Vec<(String, f64)> {
    let (collapsed, _) = runs.iter().fold(
        (Vec::new(), true),
        |(mut collapsed, mut after_space), (content, size)| {
            let run: String = content
                .chars()
                .filter_map(|character| {
                    let space = character.is_whitespace();
                    let keep = !(space && after_space);
                    after_space = space;
                    keep.then_some(if space { ' ' } else { character })
                })
                .collect();
            if !run.is_empty() {
                collapsed.push((run, *size));
            }
            (collapsed, after_space)
        },
    );
    let mut collapsed: Vec<(String, f64)> = collapsed;
    while let Some((last, _)) = collapsed.last_mut() {
        let trimmed = last.trim_end().len();
        last.truncate(trimmed);
        if !last.is_empty() {
            break;
        }
        collapsed.pop();
    }
    collapsed
}

fn measure_line(document: &Document, line: &Gathering, runs: &[(String, f64)]) -> Line {
    let size = runs
        .iter()
        .map(|(_, size)| *size)
        .reduce(f64::max)
        .unwrap_or_else(|| font_size(document, &line.path));
    let width = runs.iter().map(|(content, size)| text_width(content, *size)).sum();
    let offset = translation(document, &line.path);
    Line {
        path: line.path.clone(),
        bounds: measure_text(
            width,
            size,
            ascent(document, &line.path, size),
            point(line.x + offset.x, line.y + offset.y),
            inherited_property(document, &line.path, "text-anchor").unwrap_or("start"),
        ),
    }
}

/// The box around every line of a `text` element.
pub fn text_bounds(document: &Document, text: &[usize]) -> Bounds {
    enclosing(&lines(document, text).iter().map(|line| line.bounds).collect::<Vec<_>>())
        .expect("a text has at least one line")
}

/// The id an element names a label by: its `data-label`, else its `id`.
fn label_id(element: &Element) -> Option<&str> {
    non_empty(element.attr("data-label")).or_else(|| non_empty(element.attr("id")))
}

/// The texts making up a label: those inside a group carrying the id, or
/// else every text naming it.
pub fn label_texts(document: &Document, id: &str) -> Vec<Path> {
    let in_group = document
        .elements_named("g")
        .into_iter()
        .find(|(_, group)| non_empty(group.attr("data-label")) == Some(id))
        .map(|(group, _)| {
            document
                .elements_named("text")
                .into_iter()
                .filter(|(text, _)| text.len() > group.len() && text.starts_with(&group))
                .map(|(text, _)| text)
                .collect::<Vec<_>>()
        });
    in_group.unwrap_or_else(|| {
        document
            .elements_named("text")
            .into_iter()
            .filter(|(_, text)| label_id(text) == Some(id))
            .map(|(text, _)| text)
            .collect()
    })
}

/// The box around every line of a label's texts.
pub fn label_bounds(document: &Document, texts: &[Path]) -> Option<Bounds> {
    enclosing(&texts.iter().map(|text| text_bounds(document, text)).collect::<Vec<_>>())
}

/// A written coordinate without the float noise that shifting leaves.
fn tidy(value: f64) -> f64 {
    round(value * 1000.0) / 1000.0
}

/// Moves a label's texts so their box goes from `from` to `to`, aligning
/// every line at the `anchor` edge of the new box: each line's `x` moves to
/// that edge and every `y` shifts with the box.
pub fn move_label(document: Document, texts: &[Path], from: Bounds, to: Bounds, anchor: &str) -> Document {
    let edge = match anchor {
        "middle" => to.x + to.width / 2.0,
        "end" => to.right(),
        _ => to.x,
    };
    let shift = to.y - from.y;
    texts.iter().fold(document, |document, text| {
        let starts: Vec<Path> = lines(&document, text).into_iter().map(|line| line.path).collect();
        let positioned: Vec<Path> = document
            .elements()
            .into_iter()
            .filter(|(path, element)| {
                path.starts_with(text) && (path.len() == text.len() || element.has_attr("x") || element.has_attr("y"))
            })
            .map(|(path, _)| path)
            .collect();
        positioned.iter().fold(document, |document, path| {
            let offset = translation(&document, path);
            let is_text = path.len() == text.len();
            let starts_line = is_text || starts.contains(path);
            document.update(path, |element| {
                let element = if starts_line {
                    element.with_attr("x", &format_number(tidy(edge - offset.x)))
                } else {
                    element
                };
                let element = if is_text || element.has_attr("y") {
                    let y = number_attribute(&element, "y") + shift;
                    element.with_attr("y", &format_number(tidy(y)))
                } else {
                    element
                };
                if is_text || own_property(&element, "text-anchor").is_some() {
                    with_property(element, "text-anchor", anchor)
                } else {
                    element
                }
            })
        })
    })
}
