/-!
# Checking only the spots that can still win

`place_label` (src/route.rs) scores every spot that fits a label and keeps
the first with the best score. A spot's score is how clear it is of its
connector's crossings, bends and ends, less what other connectors near it
take off (`crowding`, a sum of `max(0.0)` terms). So the clearance alone is
a ceiling on the score.

Whether a spot fits (clear of every route, label and node) costs far more
to check than its clearance. A spot whose clearance does not exceed the
best score so far cannot win whether it fits or not, as only a strictly
better spot replaces the best. So the clearance is checked first, and such
a spot is left out. `earlyBest_eq` proves the choice stays the same.
-/

namespace Nicevg.Ceiling

variable {α : Type}

/-- One step of the fold: the best stays unless the spot scores strictly more. -/
def bestStep (score : α → Int) (best : Option (Int × α)) (x : α) : Option (Int × α) :=
  match best with
  | none => some (score x, x)
  | some (s, y) => if score x ≤ s then some (s, y) else some (score x, x)

/-- The fold over the spots that fit, as a step over every spot. -/
def fitStep (fits : α → Bool) (score : α → Int) (best : Option (Int × α)) (x : α) : Option (Int × α) :=
  if fits x then bestStep score best x else best

/-- The same step, checking the ceiling before whether the spot fits. -/
def earlyStep (fits : α → Bool) (score ceiling : α → Int) (best : Option (Int × α)) (x : α) :
    Option (Int × α) :=
  match best with
  | none => if fits x then some (score x, x) else none
  | some (s, y) =>
    if ceiling x ≤ s then some (s, y)
    else if fits x then (if score x ≤ s then some (s, y) else some (score x, x))
    else some (s, y)

theorem earlyStep_eq (fits : α → Bool) (score ceiling : α → Int) (h : ∀ x, score x ≤ ceiling x)
    (best : Option (Int × α)) (x : α) :
    earlyStep fits score ceiling best x = fitStep fits score best x := by
  cases best with
  | none => cases hf : fits x <;> simp [earlyStep, fitStep, bestStep, hf]
  | some b =>
    obtain ⟨s, y⟩ := b
    have := h x
    by_cases hc : ceiling x ≤ s
    · have hs : score x ≤ s := by omega
      cases hf : fits x <;> simp [earlyStep, fitStep, bestStep, hf, hc, hs]
    · cases hf : fits x <;> simp [earlyStep, fitStep, bestStep, hf, hc]

theorem fold_fits (fits : α → Bool) (score : α → Int) :
    ∀ (xs : List α) (best : Option (Int × α)),
      (xs.filter fits).foldl (bestStep score) best = xs.foldl (fitStep fits score) best := by
  intro xs
  induction xs with
  | nil => intro best; rfl
  | cons x xs ih =>
    intro best
    cases hf : fits x
    · simp [hf, fitStep, ih]
    · simp [hf, fitStep, ih]

/-- **Checking the ceiling first changes no choice** among the spots that fit. -/
theorem earlyBest_eq (fits : α → Bool) (score ceiling : α → Int) (h : ∀ x, score x ≤ ceiling x)
    (xs : List α) :
    xs.foldl (earlyStep fits score ceiling) none = (xs.filter fits).foldl (bestStep score) none := by
  have : earlyStep fits score ceiling = fitStep fits score := by
    funext best x; exact earlyStep_eq fits score ceiling h best x
  rw [this, fold_fits]

/-- A label spot's clearance is a ceiling on its score. -/
theorem clearance_is_ceiling (clearance crowding : Int) (h : 0 ≤ crowding) : clearance - crowding ≤ clearance := by
  omega

end Nicevg.Ceiling
