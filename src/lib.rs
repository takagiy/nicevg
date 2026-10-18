//! Deterministic checks and repairs for SVG diagrams.
//!
//! [`analyze`] reads a diagram and reports mechanical layout errors;
//! [`fix`] repairs the ones it can, and [`arrange`] also nudges nodes so
//! connectors bend, cross and crowd less. All three are pure functions of the
//! SVG text. A diagram may also be given as a [`Sketch`], nodes and connectors
//! as data, through [`analyze_sketch`], [`fix_sketch`] and [`arrange_sketch`].

mod arrange;
mod arrow;
mod diagram;
mod fix;
mod geometry;
mod inspect;
mod pathdata;
mod route;
mod sketch;
mod text;
mod xml;

use serde::Serialize;

pub use arrange::arrange;
#[cfg(feature = "screening")]
pub use arrange::screening;
#[cfg(feature = "trace-candidates")]
pub use arrange::trace as candidate_trace;
pub use diagram::{Diagram, DiagramConnector, DiagramLabel, DiagramNode, Port, Shape, UnsupportedElement};
pub use fix::{FixChange, FixResult, fix, fix_with_passes};
pub use geometry::{Bounds, Point};
pub use inspect::DiagramIssue;
pub use route::Side;
pub use sketch::{
    Sketch, SketchConnector, SketchError, SketchNode, SketchShape, analyze_sketch, arrange_sketch, fix_sketch,
    sketch_svg,
};

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

#[tracing::instrument(skip_all)]
pub(crate) fn report_of(document: &xml::Document) -> AnalysisReport {
    let diagram = tracing::info_span!("read_diagram").in_scope(|| diagram::read(document));
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
#[tracing::instrument(skip_all)]
pub fn analyze(svg: &str) -> Result<AnalysisReport, SvgInputError> {
    parse(svg).map(|document| report_of(&document))
}
