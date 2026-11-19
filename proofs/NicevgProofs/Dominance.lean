/-!
# A state reached with more bends for more is never on a cheapest route

`astar` (src/route.rs) keeps a search state per grid point, axis of arrival
and bends made (up to `EASY_BENDS`). Before it expands a state, it looks at
the states at the same point and axis with fewer bends: if one of them was
reached for less, by more than their difference in running alongside a
edge, the state is skipped.

`cheaper_with_fewer_bends` is why that loses nothing. From the state with
fewer bends, the same moves lead through the same points, and cost no more
than from the skipped state, but for that difference. So every route
through the skipped state has a strictly cheaper one with the same moves
(`dominated_never_cheapest`), and no cheapest route is lost.

## Model

* A `Place` is a grid point with the axis of arrival; what a move leads to
  (`next`: nothing when it is blocked, outside the corridor or arrives
  backwards at an end) depends on the place alone.
* A step costs, as in the search: what depends on the place and move alone
  (`base`: length, pushes, crossings, near misses and an end's own cost),
  a bend at `tc bent`, the bend onto an end stub at `tc` of the bends
  after the move, and running alongside a edge, paid by how much
  closer the step runs than the one before, which a bend forgets.
* `tc` never falls as bends are made (`bends_cost`: the bend cost, three
  times as much from `EASY_BENDS` on). Capping the bends kept changes no
  bend's cost (`tc_cap` in Admissible.lean), so the model keeps them all.
-/

namespace Nicevg.Dominance

/-- `max(0.0)` -/
def pos (x : Int) : Int := if x < 0 then 0 else x

theorem pos_nonneg (x : Int) : 0 ≤ pos x := by unfold pos; split <;> omega

theorem pos_self : pos (x - x) = 0 := by unfold pos; split <;> omega

/-- Paying for running closer than `b` costs at most paying for running
closer than `a` and for `a` over `b`. -/
theorem pos_sub_le (n a b : Int) : pos (n - b) ≤ pos (n - a) + pos (a - b) := by
  unfold pos; repeat' split
  all_goals omega

structure Costs (Place Move : Type) where
  next : Place → Move → Option Place
  bends : Place → Move → Bool
  base : Place → Move → Int
  /-- What running alongside a edge on the move's link costs. -/
  near : Place → Move → Int
  /-- Whether the move arrives at an end across its stub, bending once more. -/
  endBend : Place → Move → Bool
  tc : Nat → Int
  tc_mono : ∀ a b, a ≤ b → tc a ≤ tc b

/-- A search state, with what its distance depends on beyond the place: the
bends made and how close the step into it ran alongside a edge. -/
structure State (Place : Type) where
  place : Place
  bent : Nat
  nearBefore : Int

variable {Place Move : Type}

def bentAfter (c : Costs Place Move) (s : State Place) (m : Move) : Nat :=
  if c.bends s.place m then s.bent + 1 else s.bent

def stepCost (c : Costs Place Move) (s : State Place) (m : Move) : Int :=
  c.base s.place m
    + (if c.bends s.place m then c.tc s.bent else 0)
    + (if c.endBend s.place m then c.tc (bentAfter c s m) else 0)
    + pos (c.near s.place m - (if c.bends s.place m then 0 else s.nearBefore))

/-- What the moves cost from a state, if each can be made. -/
def cost (c : Costs Place Move) : State Place → List Move → Option Int
  | _, [] => some 0
  | s, m :: ms =>
    match c.next s.place m with
    | none => none
    | some p => (cost c ⟨p, bentAfter c s m, c.near s.place m⟩ ms).map (stepCost c s m + ·)

/-- A step from a state with no more bends costs no more, but for how much
closer the other state's last step ran alongside a edge. -/
theorem stepCost_le (c : Costs Place Move) (p : Place) {td tq : Nat} (nd nq : Int) (h : td ≤ tq) (m : Move) :
    stepCost c ⟨p, td, nd⟩ m ≤ stepCost c ⟨p, tq, nq⟩ m + pos (nq - nd) := by
  have h1 := c.tc_mono td tq h
  have h2 := c.tc_mono (td + 1) (tq + 1) (by omega)
  have h3 := pos_sub_le (c.near p m) nq nd
  have h4 := pos_nonneg (nq - nd)
  simp only [stepCost, bentAfter]
  cases c.bends p m <;> cases c.endBend p m <;> simp <;> omega

/-- From a state at the same place with no more bends, the same moves can be
made and cost no more, but for the difference in running alongside. -/
theorem cheaper_with_fewer_bends (c : Costs Place Move) :
    ∀ (ms : List Move) (p : Place) (td tq : Nat) (nd nq : Int) (C : Int), td ≤ tq →
      cost c ⟨p, tq, nq⟩ ms = some C →
      ∃ C', cost c ⟨p, td, nd⟩ ms = some C' ∧ C' ≤ C + pos (nq - nd) := by
  intro ms
  induction ms with
  | nil =>
    intro p td tq nd nq C _ hC
    have := pos_nonneg (nq - nd)
    simp only [cost, Option.some.injEq] at hC ⊢
    exact ⟨0, rfl, by omega⟩
  | cons m ms ih =>
    intro p td tq nd nq C h hC
    simp only [cost] at hC ⊢
    cases hn : c.next p m with
    | none => simp [hn] at hC
    | some p' =>
      simp only [hn] at hC ⊢
      cases hq : cost c ⟨p', bentAfter c ⟨p, tq, nq⟩ m, c.near p m⟩ ms with
      | none => simp [hq] at hC
      | some C1 =>
        simp only [hq, Option.map_some, Option.some.injEq] at hC
        have hturned : bentAfter c ⟨p, td, nd⟩ m ≤ bentAfter c ⟨p, tq, nq⟩ m := by
          simp only [bentAfter]; split <;> omega
        obtain ⟨C1', hd, hle⟩ := ih p' _ _ (c.near p m) (c.near p m) C1 hturned hq
        rw [pos_self] at hle
        have hstep := stepCost_le c p nd nq h m
        refine ⟨stepCost c ⟨p, td, nd⟩ m + C1', by simp [hd], by omega⟩

/-- **A dominated state is never on a cheapest route.** If a state with no
more bends at the same place was reached for less, by more than the
difference in running alongside, then whatever moves finish a route from
the dominated state finish a strictly cheaper one from the other. -/
theorem dominated_never_cheapest (c : Costs Place Move) (p : Place) {td tq : Nat} (nd nq gd gq : Int)
    (h : td ≤ tq) (hg : gd + pos (nq - nd) < gq) (ms : List Move) (C : Int)
    (hC : cost c ⟨p, tq, nq⟩ ms = some C) : ∃ C', cost c ⟨p, td, nd⟩ ms = some C' ∧ gd + C' < gq + C := by
  obtain ⟨C', hd, hle⟩ := cheaper_with_fewer_bends c ms p td tq nd nq C h hC
  exact ⟨C', hd, by omega⟩

end Nicevg.Dominance
