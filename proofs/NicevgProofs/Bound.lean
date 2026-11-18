/-!
A lower bound on the arrange score that needs no fix run.

`score` (src/arrange.rs) adds the movement, each connector's `route_cost`
(its length plus its turn costs) and terms that are never negative. A
connector runs from a point in one node's box to a point in another's, so
its length is at least the gap between the two boxes. Hence

  movement + Σ boxGap(from, to) ≤ score

and a candidate whose bound is no less than the best score so far cannot
be chosen.

Model: coordinates are integers, so f64 rounding is not covered.
-/

namespace Nicevg

def absI (x : Int) : Int := if x < 0 then -x else x

theorem le_absI (x : Int) : x ≤ absI x ∧ -x ≤ absI x := by
  unfold absI; split <;> omega

theorem absI_le {x y : Int} (h1 : x ≤ y) (h2 : -x ≤ y) : absI x ≤ y := by
  unfold absI; split <;> omega

structure Point where
  x : Int
  y : Int

/-- The length of a segment, as `route_cost` measures it. -/
def dist (a b : Point) : Int := absI (a.x - b.x) + absI (a.y - b.y)

theorem dist_triangle (a b c : Point) : dist a c ≤ dist a b + dist b c := by
  unfold dist
  obtain ⟨h1, h1'⟩ := le_absI (a.x - b.x)
  obtain ⟨h2, h2'⟩ := le_absI (b.x - c.x)
  obtain ⟨h3, h3'⟩ := le_absI (a.y - b.y)
  obtain ⟨h4, h4'⟩ := le_absI (b.y - c.y)
  have hx : absI (a.x - c.x) ≤ absI (a.x - b.x) + absI (b.x - c.x) := absI_le (by omega) (by omega)
  have hy : absI (a.y - c.y) ≤ absI (a.y - b.y) + absI (b.y - c.y) := absI_le (by omega) (by omega)
  omega

/-- The length of a route: the sum of its segments. -/
def length : List Point → Int
  | a :: b :: rest => dist a b + length (b :: rest)
  | _ => 0

/-- The last point of `a :: rest`. -/
def lastOr (a : Point) : List Point → Point
  | [] => a
  | b :: rest => lastOr b rest

/-- A route is no shorter than the straight distance between its ends. -/
theorem dist_le_length (a : Point) (rest : List Point) : dist a (lastOr a rest) ≤ length (a :: rest) := by
  induction rest generalizing a with
  | nil => simp [dist, absI, lastOr, length]
  | cons b rest ih =>
    have h := dist_triangle a b (lastOr b rest)
    have h' := ih b
    simp only [lastOr, length]
    omega

structure Box where
  x : Int
  y : Int
  w : Int
  h : Int

def Box.contains (box : Box) (p : Point) : Prop :=
  box.x ≤ p.x ∧ p.x ≤ box.x + box.w ∧ box.y ≤ p.y ∧ p.y ≤ box.y + box.h

/-- The gap between two intervals, 0 when they overlap. -/
def gap1 (lo1 hi1 lo2 hi2 : Int) : Int :=
  if hi1 < lo2 then lo2 - hi1 else if hi2 < lo1 then lo1 - hi2 else 0

theorem gap1_le {lo1 hi1 lo2 hi2 p q : Int}
    (hp : lo1 ≤ p ∧ p ≤ hi1) (hq : lo2 ≤ q ∧ q ≤ hi2) : gap1 lo1 hi1 lo2 hi2 ≤ absI (p - q) := by
  obtain ⟨h1, h1'⟩ := le_absI (p - q)
  unfold gap1
  repeat' split
  all_goals omega

/-- The gap between two boxes, across plus down. -/
def boxGap (a b : Box) : Int :=
  gap1 a.x (a.x + a.w) b.x (b.x + b.w) + gap1 a.y (a.y + a.h) b.y (b.y + b.h)

theorem boxGap_le_dist {a b : Box} {p q : Point} (hp : a.contains p) (hq : b.contains q) :
    boxGap a b ≤ dist p q := by
  obtain ⟨px1, px2, py1, py2⟩ := hp
  obtain ⟨qx1, qx2, qy1, qy2⟩ := hq
  have hx := gap1_le (lo1 := a.x) (hi1 := a.x + a.w) (lo2 := b.x) (hi2 := b.x + b.w) ⟨px1, px2⟩ ⟨qx1, qx2⟩
  have hy := gap1_le (lo1 := a.y) (hi1 := a.y + a.h) (lo2 := b.y) (hi2 := b.y + b.h) ⟨py1, py2⟩ ⟨qy1, qy2⟩
  unfold boxGap dist
  omega

/-- A drawn connector: its route starts in `src`'s box and ends in `dst`'s. -/
structure Connector where
  src : Box
  dst : Box
  start : Point
  rest : List Point
  starts : src.contains start
  ends : dst.contains (lastOr start rest)

theorem boxGap_le_length (c : Connector) : boxGap c.src c.dst ≤ length (c.start :: c.rest) :=
  Int.le_trans (boxGap_le_dist c.starts c.ends) (dist_le_length c.start c.rest)

def sumGap : List Connector → Int
  | [] => 0
  | c :: cs => boxGap c.src c.dst + sumGap cs

def sumLength : List Connector → Int
  | [] => 0
  | c :: cs => length (c.start :: c.rest) + sumLength cs

theorem sumGap_le_sumLength : ∀ cs : List Connector, sumGap cs ≤ sumLength cs
  | [] => by simp [sumGap, sumLength]
  | c :: cs => by
    have h1 := boxGap_le_length c
    have h2 := sumGap_le_sumLength cs
    simp only [sumGap, sumLength]
    omega

/-- `rest` stands for every other term of `score`: issues, turns, crossings,
alignment and so on. Each is a count or a cost that is never negative. -/
theorem lowerBound_le_score (movement rest : Int) (cs : List Connector) (h : 0 ≤ rest) :
    movement + sumGap cs ≤ movement + sumLength cs + rest := by
  have := sumGap_le_sumLength cs
  omega

end Nicevg
