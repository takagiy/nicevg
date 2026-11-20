/-!
# Candidates rerouting the same edges share one evaluation

Layout tries candidates that reroute the edges of some nodes in
place (`evaluate` with a zero step, src/layout.rs). Such a candidate hands
fixing the state's own document with the nodes it names marked as moved.
The marks are read in one place only: `pick_reroutes` (src/fix.rs), which
reroutes the edges with an issue and those with an end at a marked
node or inside one. The steps before it read no mark, so they change the
draft alike for every candidate, and everything after works from the
edges picked.

So two candidates that mark the same edges (`rerouted_with`) are
evaluated to the same result, and `evaluate_all` evaluates only the first
of them. `same_marked_same_picked` is the first half: marking the same
edges picks the same edges. `shared_eq` is the second: giving
every candidate the result of the first one with its key gives each its
own result.

Checked in the Rust code (2026-11-19): `moved_node_ids` is read only by
`pick_reroutes`. `separate_nodes` adds the nodes it moves to the marks, the
same ones for every candidate, which marks the same edges more for
each. Whatever takes a state's draft further
(`evaluate`, `fix_further`, `spread_parallel_runs`) sets the marks anew.
-/

namespace Nicevg.Shared

variable {α β κ γ : Type}

/-- Two filters keeping the same elements of a list agree on each of them. -/
theorem agree_of_filter_eq (p q : α → Bool) :
    ∀ l : List α, l.filter p = l.filter q → ∀ x ∈ l, p x = q x := by
  intro l
  induction l with
  | nil => intro _ x hx; cases hx
  | cons a l ih =>
    intro h x hx
    have keeps : ∀ (r : α → Bool) (y : α) (m : List α), y ∈ m.filter r → r y = true :=
      fun r y m hy => (List.mem_filter.mp hy).2
    cases hp : p a <;> cases hq : q a
    · simp only [List.filter_cons, hp, hq] at h
      rcases List.mem_cons.mp hx with rfl | hx
      · rw [hp, hq]
      · exact ih h x hx
    · have h' : l.filter p = a :: l.filter q := by simpa [List.filter_cons, hp, hq] using h
      have : a ∈ l.filter p := by rw [h']; exact List.mem_cons_self
      rw [keeps p a l this] at hp; cases hp
    · have h' : a :: l.filter p = l.filter q := by simpa [List.filter_cons, hp, hq] using h
      have : a ∈ l.filter q := by rw [← h']; exact List.mem_cons_self
      rw [keeps q a l this] at hq; cases hq
    · simp only [List.filter_cons, hp, hq, ite_true, List.cons.injEq, true_and] at h
      rcases List.mem_cons.mp hx with rfl | hx
      · rw [hp, hq]
      · exact ih h x hx

/-- The edges `pick_reroutes` picks: those with an issue rerouting
fixes and those `marked`, with an end at a moved node or inside one. -/
def picked (issue marked : γ → Bool) (edges : List γ) : List γ :=
  edges.filter (fun c => issue c || marked c)

/-- Marking the same edges picks the same edges. -/
theorem same_marked_same_picked (issue m1 m2 : γ → Bool) (edges : List γ)
    (h : edges.filter m1 = edges.filter m2) :
    picked issue m1 edges = picked issue m2 edges := by
  have agree := agree_of_filter_eq m1 m2 edges h
  unfold picked
  apply List.filter_congr
  intro c hc
  rw [agree c hc]

/-- Each candidate given the result of the first one with its key. -/
def shared [BEq κ] (key : α → κ) (f : α → β) (xs : List α) : List β :=
  xs.map (fun x => match xs.find? (fun y => key y == key x) with
    | some y => f y
    | none => f x)

/-- **Sharing changes no result**, when candidates with one key evaluate
alike. -/
theorem shared_eq [BEq κ] [LawfulBEq κ] (key : α → κ) (f : α → β) (xs : List α)
    (h : ∀ x y, key x = key y → f x = f y) : shared key f xs = xs.map f := by
  unfold shared
  apply List.map_congr_left
  intro x _
  cases hfind : xs.find? (fun y => key y == key x) with
  | none => rfl
  | some y =>
    have := List.find?_some hfind
    exact h y x (by simpa using this)

/-- Together: the result is worked out from the edges picked, so the
candidates may share by the edges they mark. -/
theorem shared_by_marked [DecidableEq γ] (issue : γ → Bool) (edges : List γ) (marks : α → γ → Bool)
    (result : List γ → β) (xs : List α) :
    shared (fun x => edges.filter (marks x)) (fun x => result (picked issue (marks x) edges)) xs
      = xs.map (fun x => result (picked issue (marks x) edges)) := by
  apply shared_eq
  intro x y hkey
  rw [same_marked_same_picked issue (marks x) (marks y) edges hkey]

end Nicevg.Shared
