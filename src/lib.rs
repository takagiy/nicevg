//! Deterministic checks and repairs for SVG diagrams.
//!
//! [`analyze`] reads a diagram and reports mechanical layout errors;
//! [`fix`] repairs the ones it can. Both are pure functions of the SVG text.

mod arrow;
mod diagram;
mod fix;
mod geometry;
mod inspect;
mod pathdata;
mod route;
mod text;
mod xml;

use serde::Serialize;

pub use diagram::{Diagram, DiagramConnector, DiagramLabel, DiagramNode, Shape, UnsupportedElement};
pub use fix::{FixChange, FixResult, fix, fix_with_passes};
pub use geometry::{Bounds, Point};
pub use inspect::DiagramIssue;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReport {
    pub valid: bool,
    pub issues: Vec<DiagramIssue>,
    pub drawing_bounds: Option<Bounds>,
    pub diagram: Diagram,
}

/// The input is not a well-formed SVG document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgInputError(pub String);

impl std::fmt::Display for SvgInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SvgInputError {}

pub(crate) fn parse(svg: &str) -> Result<xml::Document, SvgInputError> {
    xml::parse(svg).map_err(|error| SvgInputError(error.0))
}

pub(crate) fn report_of(document: &xml::Document) -> AnalysisReport {
    let diagram = diagram::read(document);
    let view_box = inspect::parse_view_box(document.root().attr("viewBox"));
    let issues = inspect::inspect(view_box, &diagram);
    AnalysisReport {
        valid: issues.is_empty(),
        issues,
        drawing_bounds: diagram.drawing_bounds(),
        diagram,
    }
}

/// Reads an SVG diagram and reports its mechanical layout errors.
pub fn analyze(svg: &str) -> Result<AnalysisReport, SvgInputError> {
    parse(svg).map(|document| report_of(&document))
}
