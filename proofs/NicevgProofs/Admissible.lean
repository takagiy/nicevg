import NicevgProofs.Bound

/-!
# The route search's estimate never overestimates

`search_along_rough_route` (src/route.rs) is A*: a queued state's priority
is its cost so far plus `estimate`, a bound on what is left to pay. A*
returns the cheapest route only if that bound never exceeds the cost of
any way left to an end: the estimate is *admissible*. `estimate_le` proves
that for a model of the search.

## Model

* The grid: point `(xi, yi)` lies at `(X xi, Y yi)`, both strictly
  increasing, as `xs` and `ys` are sorted and distinct. Integers stand for
  `f64`, so rounding is not covered.
* A way left from a state: the cells it steps through, each a neighbour of
  the one before, ending at the first end it reaches (the search stops
  there) and not arriving backwards (`arrives_backwards`).
* Its cost, `lowCost`: the end terminal's cost, the length, the bends and
  the crossings, each as the search adds them. The search adds more on a
  step (`pushes`, running close, near misses, 1.01 on outlines); taking
  each to be never negative, a way costs at least `lowCost`.
* Link weights (`Weights`): a count per grid link, for the links a route
  cannot take (`blocked_horizontal`, `blocked_vertical`) and those that
  cross an edge (`crossing_horizontal`, `crossing_vertical`). `pre` is
  the running count `along_rows` and `down_columns` build, and
  `absI (run ..)` the difference `run_counts` takes.
* Bends: a state keeps at most `EASY_BENDS` of them, which `bends_cost`
  cannot tell from more (`tc_cap`), so the model keeps them all.

## Assumptions checked in the Rust code (2026-11-19)

* `X`, `Y` strictly increasing: `xs` and `ys` come from `sorted_distinct`,
  and lanes are inserted at their place more than `LINE_MERGE` from their
  neighbours (`axis_lines`). Numbers read from a diagram are finite
  (`geometry::within`), so no NaN breaks the order.
* What a step adds beyond `lowCost` is never negative: `pushes` is 0 or
  a sum of `max(0)` terms times `CROWDED_PUSH_COST` (`push_cost` in
  src/fix.rs); running close is `(near - near_before).max(0.0)`; near
  misses cost `NEAR_MISS_COST`; outlines scale a length by 1.01.
* `bend` is `BEND_COST` or `SIDE_BEND_COST`, both positive.
* An end terminal's cost is not negative (needed with more than
  `MAX_ENDS` ends, whose estimate leaves it out): `end_terminals` gives
  `END_STUBS[0] - length + short` with `length` one of `END_STUBS`, and
  `choose_sides` gives 0 or `SIDE_CHANGE_COST` plus `CROWDING_COST` times
  a side's load. A load counts the edge ends on the side and takes
  off exactly what the edge's own previous choice added (`load_of`
  in src/fix.rs), so it is never negative.
-/

namespace Nicevg.Route

open Nicevg (absI le_absI absI_le)

theorem absI_nonneg (x : Int) : 0 ≤ absI x := by unfold absI; split <;> omega

theorem absI_add (x y : Int) : absI (x + y) ≤ absI x + absI y := by
  obtain ⟨h1, h2⟩ := le_absI x
  obtain ⟨h3, h4⟩ := le_absI y
  exact absI_le (by omega) (by omega)

/-- If `a'` and `a` have one sign and `a' * q` is not negative, neither is `a * q`. -/
theorem sign_same {a a' q : Int} (h : (0 < a ∧ 0 < a') ∨ (a < 0 ∧ a' < 0)) (h' : 0 ≤ a' * q) :
    0 ≤ a * q := by
  rcases h with ⟨ha, ha'⟩ | ⟨ha, ha'⟩
  · by_cases hq : 0 ≤ q
    · exact Int.mul_nonneg (by omega) hq
    · have := Int.mul_neg_of_pos_of_neg ha' (by omega : q < 0); omega
  · by_cases hq : q ≤ 0
    · have := Int.mul_nonneg (by omega : 0 ≤ -a) (by omega : 0 ≤ -q)
      rw [Int.neg_mul_neg] at this; exact this
    · have := Int.mul_neg_of_neg_of_pos ha' (by omega : 0 < q); omega

/-- An axis: `false` along a row, `true` down a column, as `axis` 0 and 1. -/
abbrev Axis := Bool

@[ext] structure Cell where
  x : Nat
  y : Nat
deriving DecidableEq

def along : Axis → Cell → Nat
  | false, c => c.x
  | true, c => c.y

def across : Axis → Cell → Nat
  | false, c => c.y
  | true, c => c.x

/-- A move to a neighbouring grid point. -/
def Step (a b : Cell) : Prop :=
  (a.y = b.y ∧ (b.x = a.x + 1 ∨ a.x = b.x + 1)) ∨ (a.x = b.x ∧ (b.y = a.y + 1 ∨ a.y = b.y + 1))

/-- The axis of a move, `vertical` in the search: vertical when x stays. -/
def axisOf (a b : Cell) : Axis := decide (a.x = b.x)

theorem step_along {c b : Cell} {v : Axis} (h : Step c b) (hv : axisOf c b = v) :
    across v b = across v c ∧ (along v b = along v c + 1 ∨ along v c = along v b + 1) := by
  cases v <;> simp [axisOf, Step, along, across] at * <;> omega

def IsWalk : Cell → List Cell → Prop
  | _, [] => True
  | a, b :: l => Step a b ∧ IsWalk b l

def last : Cell → List Cell → Cell
  | a, [] => a
  | _, b :: l => last b l

/-- The cell a way leaves for its last one. -/
def prevOf : Cell → List Cell → Cell
  | a, [] => a
  | a, [_] => a
  | _, b :: c :: l => prevOf b (c :: l)

/-- No cell before the last is `d`. -/
def Avoids (d : Cell) : Cell → List Cell → Prop
  | _, [] => True
  | a, b :: l => a ≠ d ∧ Avoids d b l

/-- Every move is along `v`. -/
def Straight (v : Axis) : Cell → List Cell → Prop
  | _, [] => True
  | a, b :: l => axisOf a b = v ∧ Straight v b l

/-! ## Counts along a line -/

/-- A count per grid link: by axis, the line it lies on and its lower end. -/
abbrev Weights := Axis → Nat → Nat → Nat

def stepW (w : Weights) (a b : Cell) : Nat :=
  w (axisOf a b) (across (axisOf a b) a) (min (along (axisOf a b) a) (along (axisOf a b) b))

def walkW (w : Weights) : Cell → List Cell → Nat
  | _, [] => 0
  | a, b :: l => stepW w a b + walkW w b l

/-- The running count up to point `i` of line `r`. -/
def pre (w : Weights) (v : Axis) (r : Nat) : Nat → Nat
  | 0 => 0
  | i + 1 => pre w v r i + w v r i

/-- The count on the straight run from `c` to `e` along `v`, signed. -/
def run (w : Weights) (v : Axis) (c e : Cell) : Int :=
  (pre w v (across v c) (along v e) : Int) - pre w v (across v c) (along v c)

theorem run_self (w : Weights) (v : Axis) (c : Cell) : run w v c c = 0 := by simp [run]

theorem run_trans (w : Weights) (v : Axis) {c b : Cell} (e : Cell) (h : across v b = across v c) :
    run w v c e = run w v b e + run w v c b := by
  unfold run; rw [h]; omega

theorem run_step (w : Weights) {c b : Cell} {v : Axis} (h : Step c b) (hv : axisOf c b = v) :
    absI (run w v c b) = stepW w c b := by
  obtain ⟨_, hal⟩ := step_along h hv
  unfold run stepW; rw [hv]
  rcases hal with h1 | h1
  · rw [h1, pre, show min (along v c) (along v c + 1) = along v c by omega]
    unfold absI; split <;> omega
  · rw [h1, pre, show min (along v b + 1) (along v b) = along v b by omega]
    unfold absI; split <;> omega

/-- A straight way stays on its line and takes every link of its run. -/
theorem straight_run (w : Weights) (v : Axis) : ∀ (c : Cell) (l : List Cell), IsWalk c l →
    Straight v c l → across v (last c l) = across v c ∧ absI (run w v c (last c l)) ≤ walkW w c l := by
  intro c l
  induction l generalizing c with
  | nil => intro _ _; simp [last, run, walkW, absI]
  | cons b l ih =>
    intro hw hs
    simp only [IsWalk] at hw
    simp only [Straight] at hs
    obtain ⟨hacross, _⟩ := step_along hw.1 hs.1
    obtain ⟨h1, h2⟩ := ih b hw.2 hs.2
    have hstep := run_step w hw.1 hs.1
    simp only [last, walkW]
    refine ⟨by omega, ?_⟩
    rw [run_trans w v (last b l) hacross]
    have := absI_add (run w v b (last b l)) (run w v c b)
    omega

/-- A straight way that first reaches its end there arrives from the side it
started on. -/
theorem straight_arrival (v : Axis) : ∀ (c : Cell) (l : List Cell), IsWalk c l → Straight v c l →
    Avoids (last c l) c l → c ≠ last c l →
    across v (prevOf c l) = across v (last c l) ∧
    (along v c < along v (last c l) → along v (prevOf c l) + 1 = along v (last c l)) ∧
    (along v (last c l) < along v c → along v (prevOf c l) = along v (last c l) + 1) := by
  intro c l
  induction l generalizing c with
  | nil => intro _ _ _ h; simp [last] at h
  | cons b l ih =>
    intro hw hs hav _
    simp only [IsWalk] at hw
    simp only [Straight] at hs
    obtain ⟨hacross, hal⟩ := step_along hw.1 hs.1
    cases l with
    | nil =>
      simp only [last, prevOf]
      exact ⟨hacross.symm, by omega, by omega⟩
    | cons b' l' =>
      simp only [Avoids, last] at hav
      have hacl := (straight_run (fun _ _ _ => 0) v b (b' :: l') hw.2 hs.2).1
      simp only [last] at hacl
      obtain ⟨hpa, hlt, hgt⟩ := ih b hw.2 hs.2 (by simp only [last]; exact hav.2) (by simp only [last]; exact hav.2.1)
      simp only [last] at hpa hlt hgt
      have hne : along v b ≠ along v (last b' l') := by
        intro heq
        apply hav.2.1
        cases v <;> simp only [along, across] at heq hacl <;> exact Cell.ext (by omega) (by omega)
      simp only [last, prevOf]
      exact ⟨hpa, fun h => hlt (by omega), fun h => hgt (by omega)⟩

/-! ## Bends -/

/-- `bends_cost` with `EASY_BENDS` = 2 and `LATE_BEND_FACTOR` = 3. -/
def tc (p : Int) (t : Nat) : Int := if 2 ≤ t then 3 * p else p

/-- Capping the bends kept at `EASY_BENDS` changes no bend's cost. -/
theorem tc_cap (p : Int) (k j : Nat) : tc p (min k 2 + j) = tc p (k + j) := by
  unfold tc; split <;> split <;> omega

/-- The cost of `n` more bends after `t`. -/
def sumTc (p : Int) : Nat → Nat → Int
  | _, 0 => 0
  | t, n + 1 => tc p t + sumTc p (t + 1) n

theorem tc_nonneg {p : Int} (hp : 0 ≤ p) (t : Nat) : 0 ≤ tc p t := by unfold tc; split <;> omega

theorem sumTc_nonneg {p : Int} (hp : 0 ≤ p) : ∀ t n, 0 ≤ sumTc p t n
  | _, 0 => by simp [sumTc]
  | t, n + 1 => by
    have := tc_nonneg hp t
    have := sumTc_nonneg hp (t + 1) n
    simp only [sumTc]; omega

theorem sumTc_add (p : Int) : ∀ t m n, sumTc p t (m + n) = sumTc p t m + sumTc p (t + m) n
  | t, 0, n => by simp [sumTc]
  | t, m + 1, n => by
    rw [Nat.add_right_comm]
    simp only [sumTc]
    rw [sumTc_add p (t + 1) m n, show t + 1 + m = t + (m + 1) by omega]
    omega

theorem sumTc_mono {p : Int} (hp : 0 ≤ p) (t : Nat) {m n : Nat} (h : m ≤ n) :
    sumTc p t m ≤ sumTc p t n := by
  obtain ⟨d, rfl⟩ := Nat.exists_eq_add_of_le h
  rw [sumTc_add]
  have := sumTc_nonneg hp (t + m) d
  omega

/-- The bends a way makes from axis `a`, counting the one onto an end stub
along `ta`. -/
def bends (ta : Axis) : Axis → Cell → List Cell → Nat
  | a, _, [] => if a = ta then 0 else 1
  | a, c, b :: l => (if axisOf c b = a then 0 else 1) + bends ta (axisOf c b) b l

/-- The bends as the search pays them: a bend at `bends_cost(bent)`, and the
bend onto the end stub with the move into it. -/
def bendsCost (p : Int) (ta : Axis) : Axis → Nat → Cell → List Cell → Int
  | a, k, _, [] => if a = ta then 0 else tc p k
  | a, k, c, b :: l =>
    (if axisOf c b = a then 0 else tc p k) +
      bendsCost p ta (axisOf c b) (if axisOf c b = a then k else k + 1) b l

theorem bendsCost_eq (p : Int) (ta : Axis) : ∀ a k c l, bendsCost p ta a k c l = sumTc p k (bends ta a c l) := by
  intro a k c l
  induction l generalizing a k c with
  | nil => simp only [bendsCost, bends]; split <;> simp [sumTc]
  | cons b l ih =>
    simp only [bendsCost, bends]
    by_cases hv : axisOf c b = a
    · simp only [hv, ↓reduceIte, Int.zero_add, Nat.zero_add]; rw [ih]
    · simp only [hv, ↓reduceIte]
      rw [ih, show 1 + bends ta (axisOf c b) b l = bends ta (axisOf c b) b l + 1 by omega, sumTc]

theorem bends_parity (ta : Axis) : ∀ a c l, bends ta a c l % 2 = if a = ta then 0 else 1 := by
  intro a c l
  induction l generalizing a c with
  | nil => simp only [bends]; split <;> simp
  | cons b l ih =>
    simp only [bends]
    rw [Nat.add_mod, ih]
    generalize axisOf c b = v
    cases a <;> cases v <;> cases ta <;> simp

theorem bends_zero (ta : Axis) : ∀ a c l, bends ta a c l = 0 → a = ta ∧ Straight a c l := by
  intro a c l
  induction l generalizing a c with
  | nil => simp only [bends]; split <;> simp_all [Straight]
  | cons b l ih =>
    intro h
    simp only [bends] at h
    have h1 : axisOf c b = a := by
      by_cases hv : axisOf c b = a
      · exact hv
      · rw [ite_eq_right hv] at h; omega
    rw [ite_eq_left h1] at h
    obtain ⟨h2, h3⟩ := ih (axisOf c b) b (by omega)
    rw [h1] at h2 h3
    exact ⟨h2, h1, h3⟩

/-! ## The way in -/

/-- `(e - c) · (port - e)`: not negative when a move from `c` to `e` runs
towards the port, as `towards_port`, `backwards` and `arrives_backwards`
test it. -/
def towards (X Y : Nat → Int) (px py : Int) (c e : Cell) : Int :=
  (X e.x - X c.x) * (px - X e.x) + (Y e.y - Y c.y) * (py - Y e.y)

/-- A straight way that does not arrive backwards started on the side
facing away from the port. -/
theorem straight_towards {X Y : Nat → Int} (hX : ∀ i j, i < j → X i < X j) (hY : ∀ i j, i < j → Y i < Y j)
    (px py : Int) (v : Axis) (c : Cell) (l : List Cell) (hw : IsWalk c l) (hs : Straight v c l)
    (hav : Avoids (last c l) c l) (hcd : c ≠ last c l)
    (hb : 0 ≤ towards X Y px py (prevOf c l) (last c l)) : 0 ≤ towards X Y px py c (last c l) := by
  obtain ⟨hpa, hlt, hgt⟩ := straight_arrival v c l hw hs hav hcd
  have hacl := (straight_run (fun _ _ _ => 0) v c l hw hs).1
  generalize last c l = d at *
  generalize prevOf c l = e at *
  unfold towards at *
  cases v
  · simp only [along, across] at hpa hlt hgt hacl
    rw [hpa] at hb
    rw [hacl]
    simp only [Int.sub_self, Int.zero_mul, Int.add_zero] at hb ⊢
    refine sign_same ?_ hb
    have hne : c.x ≠ d.x := fun h => hcd (Cell.ext h hacl.symm)
    rcases Nat.lt_or_gt_of_ne hne with h | h
    · have := hlt h
      have := hX _ _ h
      have := hX e.x d.x (by omega)
      left; omega
    · have := hgt h
      have := hX _ _ h
      have := hX d.x e.x (by omega)
      right; omega
  · simp only [along, across] at hpa hlt hgt hacl
    rw [hpa] at hb
    rw [hacl]
    simp only [Int.sub_self, Int.zero_mul, Int.zero_add] at hb ⊢
    refine sign_same ?_ hb
    have hne : c.y ≠ d.y := fun h => hcd (Cell.ext hacl.symm h)
    rcases Nat.lt_or_gt_of_ne hne with h | h
    · have := hlt h
      have := hY _ _ h
      have := hY e.y d.y (by omega)
      left; omega
    · have := hgt h
      have := hY _ _ h
      have := hY d.y e.y (by omega)
      right; omega

/-- Where a way with one bend bends: straight along `a`, then into the end. -/
def corner (a : Axis) (c d : Cell) : Cell := if a then ⟨c.x, d.y⟩ else ⟨d.x, c.y⟩

theorem corner_self (a : Axis) (c : Cell) : corner a c c = c := by cases a <;> rfl

theorem corner_step {a : Axis} {c b : Cell} (d : Cell) (h : across a b = across a c) :
    corner a c d = corner a b d := by
  cases a <;> simp only [corner, across, Bool.false_eq_true, ↓reduceIte] at * <;> simp [h]

/-- A way that bends at its first cell is straight along the other axis, so
bends there. -/
theorem corner_of_straight {a ta : Axis} (hne : a ≠ ta) {c : Cell} {l : List Cell} (hw : IsWalk c l)
    (hs : Straight ta c l) : corner a c (last c l) = c := by
  have hacl := (straight_run (fun _ _ _ => 0) ta c l hw hs).1
  cases a <;> cases ta <;> simp_all [corner, across] <;> ext <;> simp_all

theorem other_axis {a ta v : Axis} (hne : a ≠ ta) (hv : ¬v = a) : v = ta := by
  cases a <;> cases ta <;> cases v <;> simp_all

/-- A way with one bend takes every link of the two runs through the corner. -/
theorem one_bend_runs (w : Weights) (ta : Axis) : ∀ (a : Axis) (c : Cell) (l : List Cell), a ≠ ta →
    IsWalk c l → bends ta a c l = 1 →
    absI (run w a c (corner a c (last c l))) + absI (run w ta (corner a c (last c l)) (last c l)) ≤
      walkW w c l := by
  intro a c l
  induction l generalizing c with
  | nil => intro _ _ _; simp [last, corner_self, run_self, walkW, absI]
  | cons b l ih =>
    intro hne hw ht
    have hw' := hw
    simp only [IsWalk] at hw
    simp only [bends] at ht
    by_cases hv : axisOf c b = a
    · rw [ite_eq_left hv, hv] at ht
      have ih' := ih b hne hw.2 (by omega)
      obtain ⟨hacross, _⟩ := step_along hw.1 hv
      have hstep := run_step w hw.1 hv
      simp only [last, walkW]
      rw [corner_step (last b l) hacross, run_trans w a (corner a b (last b l)) hacross]
      have := absI_add (run w a b (corner a b (last b l))) (run w a c b)
      omega
    · rw [ite_eq_right hv] at ht
      have hta := other_axis hne hv
      rw [hta] at ht
      have hs : Straight ta c (b :: l) := ⟨hta, (bends_zero ta ta b l (by omega)).2⟩
      rw [corner_of_straight hne hw' hs, run_self]
      have := (straight_run w ta c (b :: l) hw' hs).2
      simp only [absI] at this ⊢
      omega

/-- A way with one bend that does not arrive backwards does not run its last
leg away from the port. -/
theorem one_bend_towards {X Y : Nat → Int} (hX : ∀ i j, i < j → X i < X j) (hY : ∀ i j, i < j → Y i < Y j)
    (px py : Int) (ta : Axis) : ∀ (a : Axis) (c : Cell) (l : List Cell), a ≠ ta → IsWalk c l →
    bends ta a c l = 1 → Avoids (last c l) c l → l ≠ [] →
    0 ≤ towards X Y px py (prevOf c l) (last c l) →
    0 ≤ towards X Y px py (corner a c (last c l)) (last c l) := by
  intro a c l
  induction l generalizing c with
  | nil => intro _ _ _ _ h; exact absurd rfl h
  | cons b l ih =>
    intro hne hw ht hav _ hb
    have hw' := hw
    simp only [IsWalk] at hw
    simp only [bends] at ht
    by_cases hv : axisOf c b = a
    · rw [ite_eq_left hv, hv] at ht
      obtain ⟨hacross, _⟩ := step_along hw.1 hv
      simp only [last]
      rw [corner_step (last b l) hacross]
      cases l with
      | nil => simp [last, corner_self, towards]
      | cons b' l' =>
        simp only [Avoids, last] at hav
        exact ih b hne hw.2 (by omega) (by simp only [last]; exact hav.2) (by simp) (by simpa [prevOf, last] using hb)
    · rw [ite_eq_right hv] at ht
      have hta := other_axis hne hv
      rw [hta] at ht
      have hs : Straight ta c (b :: l) := ⟨hta, (bends_zero ta ta b l (by omega)).2⟩
      rw [corner_of_straight hne hw' hs]
      simp only [Avoids] at hav
      exact straight_towards hX hY px py ta c (b :: l) hw' hs (by simp only [Avoids]; exact hav) hav.1 hb

/-! ## The estimate -/

structure Terminal where
  /-- The grid point of `terminal.point`, as `end_indices`. -/
  x : Nat
  y : Nat
  axis : Axis
  px : Int
  py : Int
  cost : Int

def Terminal.cell (t : Terminal) : Cell := ⟨t.x, t.y⟩

structure Search where
  X : Nat → Int
  Y : Nat → Int
  blocked : Weights
  crossing : Weights
  /-- `bend_cost` -/
  bend : Int
  /-- `CROSSING_COST` -/
  crossingCost : Int

def len (s : Search) : Cell → List Cell → Int
  | _, [] => 0
  | a, b :: l => absI (s.X b.x - s.X a.x) + absI (s.Y b.y - s.Y a.y) + len s b l

theorem manhattan_le_len (s : Search) : ∀ (c : Cell) (l : List Cell),
    absI (s.X (last c l).x - s.X c.x) + absI (s.Y (last c l).y - s.Y c.y) ≤ len s c l := by
  intro c l
  induction l generalizing c with
  | nil => simp [last, len, absI]
  | cons b l ih =>
    have := ih b
    simp only [last, len]
    have hx := absI_add (s.X (last b l).x - s.X b.x) (s.X b.x - s.X c.x)
    have hy := absI_add (s.Y (last b l).y - s.Y b.y) (s.Y b.y - s.Y c.y)
    rw [show s.X (last b l).x - s.X b.x + (s.X b.x - s.X c.x) = s.X (last b l).x - s.X c.x by omega] at hx
    rw [show s.Y (last b l).y - s.Y b.y + (s.Y b.y - s.Y c.y) = s.Y (last b l).y - s.Y c.y by omega] at hy
    omega

/-- The least a way left from axis `a`, `k` bends in, can cost to end `t`. -/
def lowCost (s : Search) (t : Terminal) (a : Axis) (k : Nat) (c : Cell) (l : List Cell) : Int :=
  t.cost + len s c l + bendsCost s.bend t.axis a k c l + walkW s.crossing c l * s.crossingCost

/-- `on_line` -/
def OnLine (s : Search) (a : Axis) (c : Cell) (t : Terminal) : Prop :=
  if a then s.X c.x = s.X t.x else s.Y c.y = s.Y t.y

instance (s : Search) (a : Axis) (c : Cell) (t : Terminal) : Decidable (OnLine s a c t) := by
  unfold OnLine; cases a <;> infer_instance

/-- `fewest_bends` -/
def fewest (s : Search) (a : Axis) (c : Cell) (t : Terminal) : Nat :=
  if a ≠ t.axis then 1
  else if OnLine s a c t ∧ 0 ≤ towards s.X s.Y t.px t.py c t.cell then 0
  else 2

/-- The counts `run_counts` gives on the way with the fewest bends. -/
def runs (w : Weights) (a : Axis) (c : Cell) (t : Terminal) : Nat → Int
  | 0 => absI (run w a c t.cell)
  | _ => absI (run w a c (corner a c t.cell)) + absI (run w t.axis (corner a c t.cell) t.cell)

/-- `backwards`, counted with the blocked links. -/
def back (s : Search) (a : Axis) (c : Cell) (t : Terminal) : Int :=
  if fewest s a c t = 1 ∧ towards s.X s.Y t.px t.py (corner a c t.cell) t.cell < 0 then 1 else 0

/-- `detour` -/
def detour (s : Search) (a : Axis) (k : Nat) (c : Cell) (t : Terminal) : Int :=
  if fewest s a c t ≤ 1 then
    if 0 < runs s.blocked a c t (fewest s a c t) + back s a c t
    then sumTc s.bend (k + fewest s a c t) 2
    else min (sumTc s.bend (k + fewest s a c t) 2) (runs s.crossing a c t (fewest s a c t) * s.crossingCost)
  else 0

/-- `cost` in the estimate's loop: the stub, the distance and the bends. -/
def cheap (s : Search) (a : Axis) (k : Nat) (c : Cell) (t : Terminal) : Int :=
  t.cost + absI (s.X t.x - s.X c.x) + absI (s.Y t.y - s.Y c.y) + sumTc s.bend k (fewest s a c t)

/-- What the estimate takes for end `t`. -/
def estimateFor (s : Search) (a : Axis) (k : Nat) (c : Cell) (t : Terminal) : Int :=
  cheap s a k c t + detour s a k c t

theorem fewest_le (s : Search) (a : Axis) (c : Cell) (t : Terminal) : fewest s a c t ≤ 2 := by
  unfold fewest; split <;> (try split) <;> omega

theorem detour_le_two (s : Search) (a : Axis) (k : Nat) (c : Cell) (t : Terminal) :
    detour s a k c t ≤ sumTc s.bend (k + fewest s a c t) 2 ∨ fewest s a c t = 2 ∧ detour s a k c t = 0 := by
  have hf := fewest_le s a c t
  unfold detour
  split
  · left; split
    · omega
    · exact Int.min_le_left _ _
  · right; omega

theorem runs_nonneg (w : Weights) (a : Axis) (c : Cell) (t : Terminal) (f : Nat) : 0 ≤ runs w a c t f := by
  unfold runs; split
  · exact absI_nonneg _
  · have := absI_nonneg (run w a c (corner a c t.cell))
    have := absI_nonneg (run w t.axis (corner a c t.cell) t.cell); omega

theorem detour_nonneg (s : Search) (hp : 0 ≤ s.bend) (hC : 0 ≤ s.crossingCost) (a : Axis) (k : Nat)
    (c : Cell) (t : Terminal) : 0 ≤ detour s a k c t := by
  unfold detour
  have h2 := sumTc_nonneg hp (k + fewest s a c t) 2
  split
  · split
    · exact h2
    · exact Int.le_min.mpr ⟨h2, Int.mul_nonneg (runs_nonneg _ _ _ _ _) hC⟩
  · omega

theorem cheap_le_estimateFor (s : Search) (hp : 0 ≤ s.bend) (hC : 0 ≤ s.crossingCost) (a : Axis) (k : Nat)
    (c : Cell) (t : Terminal) : cheap s a k c t ≤ estimateFor s a k c t := by
  have := detour_nonneg s hp hC a k c t
  unfold estimateFor; omega

/-- With no link blocked on the runs and the last leg running towards the
port, the detour is at most the crossings on the runs. -/
theorem detour_le_crossings (s : Search) (a : Axis) (k : Nat) (c : Cell) (t : Terminal)
    (hf : fewest s a c t ≤ 1) (hz : runs s.blocked a c t (fewest s a c t) = 0)
    (htw : fewest s a c t = 1 → 0 ≤ towards s.X s.Y t.px t.py (corner a c t.cell) t.cell) :
    detour s a k c t ≤ runs s.crossing a c t (fewest s a c t) * s.crossingCost := by
  unfold detour
  rw [ite_eq_left hf, hz]
  have hback : back s a c t = 0 := by
    unfold back; split
    · rename_i h; have := htw h.1; omega
    · rfl
  rw [hback, ite_eq_right (by omega)]
  exact Int.min_le_right _ _

/-- A way the search can take from `c` to end `t`. -/
structure Way (s : Search) (t : Terminal) (c : Cell) (l : List Cell) : Prop where
  away : c ≠ t.cell
  walk : IsWalk c l
  ends : last c l = t.cell
  first : Avoids t.cell c l
  free : walkW s.blocked c l = 0
  notBackwards : 0 ≤ towards s.X s.Y t.px t.py (prevOf c l) t.cell

/-- A straight way in makes `fewest` 0. -/
theorem fewest_of_straight (s : Search) (hX : ∀ i j, i < j → s.X i < s.X j) (hY : ∀ i j, i < j → s.Y i < s.Y j)
    (t : Terminal) (a : Axis) (c : Cell) (l : List Cell) (hway : Way s t c l) (hax : a = t.axis)
    (hs : Straight a c l) : fewest s a c t = 0 := by
  obtain ⟨hc, hw, hl, hav, _, hb⟩ := hway
  have hcd : c ≠ last c l := by rw [hl]; exact hc
  have hon := (straight_run s.blocked a c l hw hs).1
  have htw := straight_towards hX hY t.px t.py a c l hw hs (by rw [hl]; exact hav) hcd (by rw [hl]; exact hb)
  rw [hl] at hon htw
  unfold fewest
  rw [ite_eq_right (by simp [hax]), ite_eq_left]
  refine ⟨?_, htw⟩
  cases a <;> simp only [across, Terminal.cell] at hon <;> simp [OnLine, hon]

/-- The estimate for one end never exceeds what a way to it costs. -/
theorem estimateFor_le (s : Search) (hX : ∀ i j, i < j → s.X i < s.X j) (hY : ∀ i j, i < j → s.Y i < s.Y j)
    (hp : 0 ≤ s.bend) (hC : 0 ≤ s.crossingCost) (t : Terminal) (a : Axis) (k : Nat) (c : Cell)
    (l : List Cell) (hway : Way s t c l) : estimateFor s a k c t ≤ lowCost s t a k c l := by
  have hway' := hway
  obtain ⟨hc, hw, hl, hav, hfree, hb⟩ := hway
  have hman := manhattan_le_len s c l
  rw [hl] at hman
  simp only [Terminal.cell] at hman
  have hne : l ≠ [] := by rintro rfl; exact hc hl
  have hpar := bends_parity t.axis a c l
  have hdn := detour_nonneg s hp hC a k c t
  have hW := Int.mul_nonneg (Int.natCast_nonneg (walkW s.crossing c l)) hC
  have hcrC : ∀ r : Int, r ≤ walkW s.crossing c l → r * s.crossingCost ≤ walkW s.crossing c l * s.crossingCost :=
    fun r hr => Int.mul_le_mul_of_nonneg_right hr hC
  unfold estimateFor cheap lowCost
  rw [bendsCost_eq]
  by_cases hax : a = t.axis
  · rw [ite_eq_left hax] at hpar
    by_cases h0 : bends t.axis a c l = 0
    · -- Straight in: the way is the straight run, so it is clear and pays its crossings.
      obtain ⟨_, hs⟩ := bends_zero t.axis a c l h0
      have hf := fewest_of_straight s hX hY t a c l hway' hax hs
      have hbl := (straight_run s.blocked a c l hw hs).2
      have hcx := (straight_run s.crossing a c l hw hs).2
      rw [hl, hfree] at hbl
      rw [hl] at hcx
      have hz : runs s.blocked a c t (fewest s a c t) = 0 := by
        rw [hf]; simp only [runs]; have := absI_nonneg (run s.blocked a c t.cell); omega
      have hd := detour_le_crossings s a k c t (by omega) hz (by omega)
      have := hcrC (runs s.crossing a c t (fewest s a c t)) (by rw [hf]; simp only [runs]; exact hcx)
      rw [hf, h0] at *
      simp only [sumTc] at *
      omega
    · -- Two bends or more pay for the two the estimate adds, if it adds them.
      have hT2 : 2 ≤ bends t.axis a c l := by omega
      have h2 := sumTc_mono hp k hT2
      have hf := fewest_le s a c t
      have hf1 : fewest s a c t ≠ 1 := by unfold fewest; rw [ite_eq_right (by simp [hax])]; split <;> simp
      rcases detour_le_two s a k c t with hd | ⟨hf2, hd⟩
      · by_cases hf0 : fewest s a c t = 0
        · rw [hf0, Nat.add_zero] at hd
          rw [hf0, show sumTc s.bend k 0 = 0 by simp [sumTc]]
          omega
        · have hd0 : detour s a k c t = 0 := by unfold detour; rw [ite_eq_right (by omega)]
          have : fewest s a c t = 2 := by omega
          rw [this, hd0]
          have := sumTc_nonneg hp k 0
          omega
      · rw [hf2, hd]
        omega
  · -- Across the end's axis: one bend or three.
    rw [ite_eq_right hax] at hpar
    have hf : fewest s a c t = 1 := by unfold fewest; simp [hax]
    by_cases h1 : bends t.axis a c l = 1
    · -- One bend: the way is the run through the corner.
      have hbl := one_bend_runs s.blocked t.axis a c l hax hw h1
      have hcx := one_bend_runs s.crossing t.axis a c l hax hw h1
      have htw := one_bend_towards hX hY t.px t.py t.axis a c l hax hw h1 (by rw [hl]; exact hav) hne
        (by rw [hl]; exact hb)
      rw [hl, hfree] at hbl
      rw [hl] at hcx htw
      have hz : runs s.blocked a c t (fewest s a c t) = 0 := by
        rw [hf]; simp only [runs]
        have := absI_nonneg (run s.blocked a c (corner a c t.cell))
        have := absI_nonneg (run s.blocked t.axis (corner a c t.cell) t.cell); omega
      have hd := detour_le_crossings s a k c t (by omega) hz (fun _ => htw)
      have := hcrC (runs s.crossing a c t (fewest s a c t)) (by rw [hf]; simp only [runs]; exact hcx)
      rw [hf, h1] at *
      simp only [sumTc] at *
      omega
    · -- Three bends or more pay for the bend and the two the estimate adds.
      have h3 : 3 ≤ bends t.axis a c l := by omega
      have := sumTc_mono hp k h3
      have := sumTc_add s.bend k 1 2
      simp only [Nat.reduceAdd] at this
      rcases detour_le_two s a k c t with hd | ⟨hf2, _⟩
      · rw [hf] at hd ⊢
        omega
      · omega

/-- The estimate's loop: an end whose cheap part already reaches the best
so far is skipped. -/
def bestOver (cheapOf totalOf : Terminal → Int) : Int → List Terminal → Int
  | best, [] => best
  | best, t :: ts =>
    if best ≤ cheapOf t then bestOver cheapOf totalOf best ts
    else bestOver cheapOf totalOf (min best (totalOf t)) ts

theorem bestOver_le (cheapOf totalOf : Terminal → Int) (h : ∀ t, cheapOf t ≤ totalOf t) :
    ∀ best ts, bestOver cheapOf totalOf best ts ≤ best ∧ ∀ t ∈ ts, bestOver cheapOf totalOf best ts ≤ totalOf t := by
  intro best ts
  induction ts generalizing best with
  | nil => simp [bestOver]
  | cons t ts ih =>
    simp only [bestOver]
    split
    · obtain ⟨h1, h2⟩ := ih best
      refine ⟨h1, ?_⟩
      intro u hu
      rcases List.mem_cons.mp hu with rfl | hu
      · have := h u; omega
      · exact h2 u hu
    · obtain ⟨h1, h2⟩ := ih (min best (totalOf t))
      have := Int.min_le_left best (totalOf t)
      have := Int.min_le_right best (totalOf t)
      refine ⟨by omega, ?_⟩
      intro u hu
      rcases List.mem_cons.mp hu with rfl | hu
      · omega
      · exact h2 u hu

theorem foldl_min_le (g : Terminal → Int) :
    ∀ (best : Int) (ts : List Terminal), ∀ t ∈ ts, ts.foldl (fun b u => min b (g u)) best ≤ g t := by
  have hle : ∀ (best : Int) (ts : List Terminal), ts.foldl (fun b u => min b (g u)) best ≤ best := by
    intro best ts
    induction ts generalizing best with
    | nil => simp
    | cons u ts ih =>
      simp only [List.foldl]
      have := ih (min best (g u)); have := Int.min_le_left best (g u); omega
  intro best ts
  induction ts generalizing best with
  | nil => simp
  | cons u ts ih =>
    intro t ht
    simp only [List.foldl]
    rcases List.mem_cons.mp ht with rfl | ht
    · have := hle (min best (g t)) ts; have := Int.min_le_right best (g t); omega
    · exact ih _ t ht

/-- `estimate` at a state off every end: `many` when there are more than
`MAX_ENDS` ends, and `top` for `f64::INFINITY`. -/
def estimate (s : Search) (ends : List Terminal) (many : Bool) (top : Int) (a : Axis) (k : Nat) (c : Cell) : Int :=
  if many then ends.foldl (fun best t => min best (absI (s.X t.x - s.X c.x) + absI (s.Y t.y - s.Y c.y))) top
  else bestOver (cheap s a k c) (estimateFor s a k c) top ends

/-- **The estimate is admissible**: at a state off every end, it never
exceeds the cost of a way the search can take to any end. -/
theorem estimate_le (s : Search) (hX : ∀ i j, i < j → s.X i < s.X j) (hY : ∀ i j, i < j → s.Y i < s.Y j)
    (hp : 0 ≤ s.bend) (hC : 0 ≤ s.crossingCost) (ends : List Terminal) (many : Bool) (top : Int)
    (t : Terminal) (ht : t ∈ ends) (hcost : 0 ≤ t.cost) (a : Axis) (k : Nat) (c : Cell) (l : List Cell)
    (hway : Way s t c l) : estimate s ends many top a k c ≤ lowCost s t a k c l := by
  unfold estimate
  split
  · have h := foldl_min_le (fun u => absI (s.X u.x - s.X c.x) + absI (s.Y u.y - s.Y c.y)) top ends t ht
    have hman := manhattan_le_len s c l
    rw [hway.ends] at hman
    simp only [Terminal.cell] at hman
    have := sumTc_nonneg hp k (bends t.axis a c l)
    have := Int.mul_nonneg (Int.natCast_nonneg (walkW s.crossing c l)) hC
    unfold lowCost; rw [bendsCost_eq]
    omega
  · have h := (bestOver_le (cheap s a k c) (estimateFor s a k c) (cheap_le_estimateFor s hp hC a k c) top ends).2 t ht
    have := estimateFor_le s hX hY hp hC t a k c l hway
    omega

end Nicevg.Route
