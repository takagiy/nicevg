/-!
# Growing the viewBox makes one check again

When the drawing does not fit the viewBox, fixing grows the viewBox, the
backgrounds drawn on the root and the SVG's size (`expand_view_box`,
src/fix.rs). The diagram read from the document is the same after that, and
of all the checks only the one that the drawing fits the viewBox reads the
viewBox (`inspect`, src/inspect.rs). So the grown draft's report is built
from the one before it: that check is made again and the other issues are
kept (`InspectionReport::with_view_box`), where the whole document used to
be read and checked again.

`inspect_again` proves the issues come out the same: the checks are the
viewBox check followed by the checks of the diagram alone, so taking the
viewBox check's issues out of a report and putting the new ones in front
gives the report under the new viewBox.

Checked in the Rust code (2026-11-19): reading a diagram takes nodes from
groups, edges from elements naming their ends and labels from texts,
and from the root only the properties texts inherit and its children's
names; it reads neither the viewBox, the root's width and height, nor the
place and size of a rect drawn on the root. No check but `viewport` gives a
`viewport-clipping` issue.
-/

namespace Nicevg.Viewport

variable {I V : Type}

/-- The checks: that the drawing fits the viewBox, then those of the diagram alone. -/
def inspect (viewport : V → List I) (rest : List I) (viewBox : V) : List I := viewport viewBox ++ rest

/-- **The report under another viewBox** is the one before with the viewBox
check's issues taken out and the new ones put in front. -/
theorem inspect_again (viewport : V → List I) (rest : List I) (isViewport : I → Bool)
    (only : ∀ v, ∀ i ∈ viewport v, isViewport i = true) (others : ∀ i ∈ rest, isViewport i = false)
    (before after : V) :
    inspect viewport rest after =
      viewport after ++ (inspect viewport rest before).filter (fun i => !isViewport i) := by
  unfold inspect
  have gone : (viewport before).filter (fun i => !isViewport i) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro i hi
    simp [only before i hi]
  have kept : rest.filter (fun i => !isViewport i) = rest := by
    apply List.filter_eq_self.mpr
    intro i hi
    simp [others i hi]
  rw [List.filter_append, gone, kept, List.nil_append]

end Nicevg.Viewport
