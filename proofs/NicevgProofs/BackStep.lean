import NicevgProofs.Dominance

/-!
# A route never gains by stepping back

`astar` (src/route.rs) does not step from a state back to the point it was
reached from. `back_step_never_cheapest` is why that loses nothing: a route
that steps to a neighbouring point and straight back costs more than the
route without those two steps, whatever moves follow.

The steps after the way back start at the same point either way, along the
axis of the two steps instead of the axis the route had before them. A
first move along that axis turns without the two steps and runs straight
with them; but the step out already paid that turn, at the same point. A
first move along the other axis runs straight without them and turns with
them. From there on the route without them has made no more turns
(`cheaper_with_fewer_turns` in Dominance.lean).

## Model

The search's costs on a grid, as a `Costs` of Dominance.lean: a place is a
point and the axis of arrival, a move a direction along an axis, and a
move turns when its axis is not the place's. A step costs what its edge
does (`edge`: length, pushes, a crossing, a near miss running past a turn,
an end's own cost), what turning onto it adds (`turnExtra`: a near miss for
turning beside another connector, unless the edge pays one already), the
turn itself, and running alongside (`near`, the same for an edge either
way). None is negative, and a near miss for turning one way at a point is
paid by turning another way there or by that way's edge (`turn_any`).
-/

namespace Nicevg.BackStep

open Nicevg.Dominance

structure Grid (Cell Dir : Type) where
  axis : Dir → Bool
  step : Cell → Dir → Option Cell
  edge : Cell → Dir → Int
  near : Cell → Dir → Int
  turnExtra : Cell → Dir → Int
  endTurn : Cell → Dir → Bool
  tc : Nat → Int
  tc_mono : ∀ a b, a ≤ b → tc a ≤ tc b
  tc_nonneg : ∀ t, 0 ≤ tc t
  edge_nonneg : ∀ c d, 0 ≤ edge c d
  near_nonneg : ∀ c d, 0 ≤ near c d
  turnExtra_nonneg : ∀ c d, 0 ≤ turnExtra c d
  /-- Turning one way at a point costs no more than turning another way
  there and taking that way's edge. -/
  turn_any : ∀ c d m, turnExtra c m ≤ turnExtra c d + edge c d

variable {Cell Dir : Type}

/-- The search's costs on the grid. -/
def Grid.costs (g : Grid Cell Dir) : Costs (Cell × Bool) Dir where
  next := fun p d => (g.step p.1 d).map (fun c => (c, g.axis d))
  turns := fun p d => p.2 != g.axis d
  base := fun p d => g.edge p.1 d + (if p.2 != g.axis d then g.turnExtra p.1 d else 0)
  near := fun p d => g.near p.1 d
  endTurn := fun p d => g.endTurn p.1 d
  tc := g.tc
  tc_mono := g.tc_mono

theorem cost_cons_some {Place Move : Type} (c : Costs Place Move) (p0 : Place) (t : Nat) (nb : Int)
    (m : Move) (ms : List Move) (C : Int) (h : cost c ⟨p0, t, nb⟩ (m :: ms) = some C) :
    ∃ p C1, c.next p0 m = some p ∧
      cost c ⟨p, turnedAfter c ⟨p0, t, nb⟩ m, c.near p0 m⟩ ms = some C1 ∧
      C = stepCost c ⟨p0, t, nb⟩ m + C1 := by
  simp only [cost] at h
  cases hn : c.next p0 m with
  | none => simp [hn] at h
  | some p =>
    simp only [hn] at h
    cases hq : cost c ⟨p, turnedAfter c ⟨p0, t, nb⟩ m, c.near p0 m⟩ ms with
    | none => simp [hq] at h
    | some C1 =>
      simp only [hq, Option.map_some, Option.some.injEq] at h
      exact ⟨p, C1, rfl, hq, h.symm⟩

theorem cost_cons_of {Place Move : Type} (c : Costs Place Move) (p0 : Place) (t : Nat) (nb : Int)
    (m : Move) (ms : List Move) (p : Place) (C1 : Int) (hn : c.next p0 m = some p)
    (hq : cost c ⟨p, turnedAfter c ⟨p0, t, nb⟩ m, c.near p0 m⟩ ms = some C1) :
    cost c ⟨p0, t, nb⟩ (m :: ms) = some (stepCost c ⟨p0, t, nb⟩ m + C1) := by
  simp [cost, hn, hq]

/-- A step costs at least its edge. -/
theorem edge_le_stepCost (g : Grid Cell Dir) (c : Cell) (a : Bool) (t : Nat) (nb : Int) (m : Dir) :
    g.edge c m ≤ stepCost g.costs ⟨(c, a), t, nb⟩ m := by
  have h1 := g.turnExtra_nonneg c m
  have h2 := g.tc_nonneg t
  have h3 := g.tc_nonneg (t + 1)
  have h4 := pos_nonneg (g.near c m - nb)
  have h5 := pos_nonneg (g.near c m)
  have h6 := pos_nonneg (g.near c m - 0)
  rcases Bool.eq_false_or_eq_true (g.axis m) with hm | hm <;>
    rcases Bool.eq_false_or_eq_true (g.endTurn c m) with he | he <;>
    cases a <;> simp [stepCost, turnedAfter, Grid.costs, hm, he] <;> omega

/-- The first move after stepping out and back costs, with the step out, at
least what it costs without the two steps. -/
theorem first_step_le (g : Grid Cell Dir) (c : Cell) (a0 : Bool) (t0 : Nat) (nb0 : Int) (hnb : 0 ≤ nb0)
    (d m : Dir) :
    stepCost g.costs ⟨(c, a0), t0, nb0⟩ m ≤
      stepCost g.costs ⟨(c, a0), t0, nb0⟩ d +
        stepCost g.costs ⟨(c, g.axis d), turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ d, g.near c d⟩ m := by
  have hn := g.near_nonneg c d
  have hnm := g.near_nonneg c m
  have he := g.edge_nonneg c d
  have hem := g.edge_nonneg c m
  have hb := g.turnExtra_nonneg c d
  have hbm := g.turnExtra_nonneg c m
  have hany := g.turn_any c d m
  have t0n := g.tc_nonneg t0
  have t1n := g.tc_nonneg (t0 + 1)
  have t2n := g.tc_nonneg (t0 + 2)
  have t2m := g.tc_nonneg (t0 + 1 + 1)
  have m01 := g.tc_mono t0 (t0 + 1) (by omega)
  have m12 := g.tc_mono (t0 + 1) (t0 + 2) (by omega)
  have m12b := g.tc_mono (t0 + 1) (t0 + 1 + 1) (by omega)
  have p1 := pos_sub_le (g.near c m) (g.near c d) nb0
  have p2 : g.near c m ≤ g.near c d + pos (g.near c m - g.near c d) := by unfold pos; split <;> omega
  have p3 : pos (g.near c m - nb0) ≤ g.near c m := by unfold pos; split <;> omega
  have p4 : pos (g.near c m) = g.near c m := by unfold pos; split <;> omega
  have p5 : pos (g.near c d) = g.near c d := by unfold pos; split <;> omega
  have p6 := pos_nonneg (g.near c m - g.near c d)
  have p7 := pos_nonneg (g.near c d - nb0)
  have p8 := pos_nonneg (g.near c m - nb0)
  rcases Bool.eq_false_or_eq_true (g.axis d) with hd | hd <;>
    rcases Bool.eq_false_or_eq_true (g.axis m) with hm | hm <;>
    rcases Bool.eq_false_or_eq_true (g.endTurn c m) with hem | hem <;>
    rcases Bool.eq_false_or_eq_true (g.endTurn c d) with hed | hed <;>
    cases a0 <;> simp [stepCost, turnedAfter, Grid.costs, hd, hm, hem, hed] <;> omega

/-- **A route that steps out and straight back is never cheapest.** The
same route without the two steps can be made and costs strictly less. -/
theorem back_step_never_cheapest (g : Grid Cell Dir) (c c1 : Cell) (a0 : Bool) (t0 : Nat) (nb0 : Int)
    (hnb : 0 ≤ nb0) (d back : Dir) (hstep : g.step c d = some c1) (hback : g.step c1 back = some c)
    (hax : g.axis back = g.axis d) (hnear : g.near c1 back = g.near c d) (hlen : 0 < g.edge c1 back)
    (ms : List Dir) (C : Int) (hC : cost g.costs ⟨(c, a0), t0, nb0⟩ (d :: back :: ms) = some C) :
    ∃ Cs, cost g.costs ⟨(c, a0), t0, nb0⟩ ms = some Cs ∧ Cs < C := by
  obtain ⟨p1, C1, hp1, hc1, hC⟩ := cost_cons_some _ _ _ _ _ _ _ hC
  have e1 : p1 = (c1, g.axis d) := by simpa [Grid.costs, hstep] using hp1.symm
  subst e1
  obtain ⟨p2, C2, hp2, hc2, hC1⟩ := cost_cons_some _ _ _ _ _ _ _ hc1
  have e2 : p2 = (c, g.axis d) := by simpa [Grid.costs, hback, hax] using hp2.symm
  subst e2
  -- The state after the way back: no turn more, beside the same edge.
  have hturned : turnedAfter g.costs
      ⟨(c1, g.axis d), turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ d, g.costs.near (c, a0) d⟩ back
      = turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ d := by
    simp [turnedAfter, Grid.costs, hax]
  have hnear2 : g.costs.near (c1, g.axis d) back = g.near c d := by simp [Grid.costs, hnear]
  rw [hturned, hnear2] at hc2
  have hsd := edge_le_stepCost g c a0 t0 nb0 d
  have hed := g.edge_nonneg c d
  have hsb := edge_le_stepCost g c1 (g.axis d) (turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ d)
    (g.costs.near (c, a0) d) back
  cases ms with
  | nil =>
    simp only [cost, Option.some.injEq] at hc2 ⊢
    exact ⟨0, rfl, by omega⟩
  | cons m rest =>
    obtain ⟨p3, C3, hp3, hc3, hC2⟩ := cost_cons_some _ _ _ _ _ _ _ hc2
    have hp3s : g.costs.next (c, a0) m = some p3 := by simpa [Grid.costs] using hp3
    have hle : turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ m ≤
        turnedAfter g.costs ⟨(c, g.axis d), turnedAfter g.costs ⟨(c, a0), t0, nb0⟩ d, g.near c d⟩ m := by
      rcases Bool.eq_false_or_eq_true (g.axis d) with hd | hd <;>
        rcases Bool.eq_false_or_eq_true (g.axis m) with hm | hm <;>
        cases a0 <;> simp [turnedAfter, Grid.costs, hd, hm] <;> omega
    obtain ⟨C3s, hc3s, hC3⟩ := cheaper_with_fewer_turns g.costs rest p3 _ _ (g.near c m) (g.near c m) C3 hle
      (by simpa [Grid.costs] using hc3)
    rw [pos_self] at hC3
    have hfirst := first_step_le g c a0 t0 nb0 hnb d m
    refine ⟨stepCost g.costs ⟨(c, a0), t0, nb0⟩ m + C3s, ?_, by omega⟩
    exact cost_cons_of _ _ _ _ _ _ p3 C3s hp3s (by simpa [Grid.costs] using hc3s)

end Nicevg.BackStep
