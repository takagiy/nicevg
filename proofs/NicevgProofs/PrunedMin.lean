import NicevgProofs.Bound

/-!
# Choosing the cheapest option without pricing every one

Several places price each of a few options with a route search and take
the first cheapest (`Iterator::min_by`). An option whose price is known,
before the search, to be no less than the best so far cannot be taken:
only a strictly cheaper option replaces the best. `prunedMin_eq` proves
that skipping such options gives the same choice.

Used for the places an end's port may take (`reroute_around` in
src/fix.rs): a route from one port to another is no shorter than the
distance between them across plus down (`dist_le_length` in Bound.lean),
its bends and what it costs the routes around it are never negative
(`route_cost`, `routes_cost`), and a route that is not orthogonal is
priced infinite. So that distance is a floor under an option's price
(`floor_le_price`). On a circle the route's end moves from the port onto
the outline (`end_on_shapes`), so there the code prices every option.
-/

namespace Nicevg.PrunedMin

variable {α : Type}

/-- One step of `min_by`: a strictly cheaper option replaces the best. -/
def step (cost : α → Int) (best : Option (Int × α)) (x : α) : Option (Int × α) :=
  match best with
  | none => some (cost x, x)
  | some (c, y) => if cost x < c then some (cost x, x) else some (c, y)

/-- The first cheapest option, as `min_by` gives it. -/
def firstMin (cost : α → Int) (xs : List α) : Option (Int × α) := xs.foldl (step cost) none

/-- The same step, not pricing an option that `skip` rules out against the
best price so far. -/
def prunedStep (cost : α → Int) (skip : Int → α → Bool) (best : Option (Int × α)) (x : α) : Option (Int × α) :=
  match best with
  | none => some (cost x, x)
  | some (c, y) => if skip c x then some (c, y) else if cost x < c then some (cost x, x) else some (c, y)

def prunedMin (cost : α → Int) (skip : Int → α → Bool) (xs : List α) : Option (Int × α) :=
  xs.foldl (prunedStep cost skip) none

theorem prunedStep_eq (cost : α → Int) (skip : Int → α → Bool)
    (sound : ∀ c x, skip c x = true → c ≤ cost x) (best : Option (Int × α)) (x : α) :
    prunedStep cost skip best x = step cost best x := by
  cases best with
  | none => rfl
  | some b =>
    obtain ⟨c, y⟩ := b
    simp only [prunedStep, step]
    cases hs : skip c x with
    | false => simp
    | true =>
      have := sound c x hs
      simp only [ite_true]
      rw [ite_eq_right (by omega)]

/-- **Skipping changes no choice**, when an option is skipped only if its
price is no less than the best so far. -/
theorem prunedMin_eq (cost : α → Int) (skip : Int → α → Bool)
    (sound : ∀ c x, skip c x = true → c ≤ cost x) (xs : List α) :
    prunedMin cost skip xs = firstMin cost xs := by
  unfold prunedMin firstMin
  have : prunedStep cost skip = step cost := by
    funext best x; exact prunedStep_eq cost skip sound best x
  rw [this]

/-- Skipping by a floor under the price: an option whose floor reaches the
best price so far, with any margin, is priced no less. -/
theorem floor_sound (cost floor : α → Int) (margin : Int) (hmargin : 0 ≤ margin)
    (hfloor : ∀ x, floor x ≤ cost x) :
    ∀ c x, decide (c + margin ≤ floor x) = true → c ≤ cost x := by
  intro c x h
  have := hfloor x
  have := of_decide_eq_true h
  omega

/-- The distance between a route's ends is a floor under its price: its
length, and bends and what it costs other routes, never negative. -/
theorem floor_le_price (start : Nicevg.Point) (rest : List Nicevg.Point) (bends others : Int)
    (hb : 0 ≤ bends) (ho : 0 ≤ others) :
    Nicevg.dist start (Nicevg.lastOr start rest) ≤ Nicevg.length (start :: rest) + bends + others := by
  have := Nicevg.dist_le_length start rest
  omega

end Nicevg.PrunedMin
