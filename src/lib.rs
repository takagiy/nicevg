//! Deterministic checks and repairs for SVG diagrams.
//!
//! [`inspect`] reads a diagram and reports mechanical layout errors;
//! [`fix`] repairs the ones it can, and [`layout`] also nudges nodes so
//! edges bend, cross and crowd less. All three are pure functions of the
//! SVG text. A diagram may also be given as a [`Sketch`], nodes and edges
//! as data, through [`inspect_sketch`], [`fix_sketch`] and [`layout_sketch`].

mod arrow;
mod diagram;
mod fix;
mod geometry;
mod inspect;
mod layout;
mod pathdata;
mod route;
mod sketch;
mod text;
mod xml;

use serde::Serialize;

pub use diagram::{Diagram, DiagramEdge, DiagramLabel, DiagramNode, Port, Shape, UnsupportedElement};
pub use fix::{FixChange, FixResult, fix, fix_with_steps};
pub use geometry::{Bounds, Point};
pub use inspect::DiagramIssue;
pub use layout::layout;
pub use route::Side;
pub use sketch::{
    PictureRequest, Sketch, SketchArrow, SketchEdge, SketchError, SketchNode, SketchShape, SketchStep, StepState,
    fix_sketch, inspect_sketch, layout_sketch, layout_sketch_in_steps, sketch_svg,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub valid: bool,
    pub issues: Vec<DiagramIssue>,
    pub diagram_bounds: Option<Bounds>,
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
pub(crate) fn report_of(document: &xml::Document) -> InspectionReport {
    report_around(document, diagram::read_nodes(document, None).nodes)
}

impl InspectionReport {
    /// The report of the same diagram under another viewBox: only the check
    /// that the drawing fits the viewBox is made again.
    pub(crate) fn with_view_box(&self, view_box: Option<Bounds>) -> InspectionReport {
        let issues: Vec<DiagramIssue> = inspect::viewport(view_box, self.diagram_bounds)
            .into_iter()
            .chain(
                self.issues
                    .iter()
                    .filter(|issue| issue.code != "diagram-clipped")
                    .cloned(),
            )
            .collect();
        InspectionReport {
            valid: issues.is_empty(),
            issues,
            diagram_bounds: self.diagram_bounds,
            diagram: self.diagram.clone(),
        }
    }
}

/// The report of a document whose nodes are already read.
pub(crate) fn report_around(document: &xml::Document, nodes: Vec<DiagramNode>) -> InspectionReport {
    let diagram = tracing::info_span!("read_diagram").in_scope(|| diagram::read_around(document, nodes));
    let view_box = inspect::parse_view_box(document.root().attr("viewBox"));
    let issues = inspect::inspect(view_box, &diagram);
    InspectionReport {
        valid: issues.is_empty(),
        issues,
        diagram_bounds: diagram.diagram_bounds(),
        diagram,
    }
}

/// Reads an SVG diagram and reports its mechanical layout errors.
#[tracing::instrument(skip_all)]
pub fn inspect(svg: &str) -> Result<InspectionReport, SvgInputError> {
    parse(svg).map(|document| report_of(&document))
}
