/-!
# Pricing the likeliest options first

The places an end's port may take are priced nearest ports first
(`reroute_around` in src/fix.rs), so the best price falls early and more of
the others are skipped (PrunedMin.lean). The choice must stay the first
cheapest in the order the places are listed. So the code keeps the option
that comes `Before` the others: cheaper, or as cheap and listed earlier.
`anyOrder_eq` proves that this gives the choice of `min_by` over the listed
order, whatever order the options are priced in, when an option is skipped
only if it is priced strictly above the best so far (the margin in the
code).
-/

namespace Nicevg.AnyOrder

variable {α : Type} (cost : α → Int) (idx : α → Nat)

/-- Cheaper, or as cheap and listed earlier. -/
def Before (x y : α) : Prop := cost x < cost y ∨ (cost x = cost y ∧ idx x < idx y)

instance (x y : α) : Decidable (Before cost idx x y) := by unfold Before; infer_instance

/-- `b` is among `seen` and none of them comes before it. -/
def Least (seen : List α) (b : α) : Prop := b ∈ seen ∧ ∀ y ∈ seen, ¬Before cost idx y b

def Inv (seen : List α) : Option α → Prop
  | none => seen = []
  | some b => Least cost idx seen b

/-- `min_by` over the listed order: a strictly cheaper option replaces the best. -/
def plainStep (best : Option α) (x : α) : Option α :=
  match best with
  | none => some x
  | some b => if cost x < cost b then some x else some b

/-- Pricing in any order: an option `skip` rules out is not priced, and one
that comes before the best replaces it. -/
def orderedStep (skip : Int → α → Bool) (best : Option α) (x : α) : Option α :=
  match best with
  | none => some x
  | some b => if skip (cost b) x then some b else if Before cost idx x b then some x else some b

theorem ordered_inv (skip : Int → α → Bool) (sound : ∀ b x, skip (cost b) x = true → cost b < cost x) :
    ∀ (l seen : List α) (best : Option α), Inv cost idx seen best →
      Inv cost idx (seen ++ l) (l.foldl (orderedStep cost idx skip) best) := by
  intro l
  induction l with
  | nil => intro seen best h; simpa using h
  | cons x l ih =>
    intro seen best h
    have key : Inv cost idx (seen ++ [x]) (orderedStep cost idx skip best x) := by
      cases best with
      | none =>
        simp only [Inv] at h
        subst h
        refine ⟨by simp, ?_⟩
        intro y hy
        simp only [List.nil_append, List.mem_singleton] at hy
        subst hy
        unfold Before; omega
      | some b =>
        obtain ⟨hb, hmin⟩ := h
        simp only [orderedStep]
        cases hs : skip (cost b) x with
        | true =>
          have := sound b x hs
          refine ⟨by simp [hb], ?_⟩
          intro y hy
          rcases List.mem_append.mp hy with hy | hy
          · exact hmin y hy
          · simp only [List.mem_singleton] at hy
            subst hy
            unfold Before; omega
        | false =>
          by_cases hbefore : Before cost idx x b
          · simp only [Bool.false_eq_true, ite_false, hbefore, ite_true]
            refine ⟨by simp, ?_⟩
            intro y hy
            rcases List.mem_append.mp hy with hy | hy
            · have := hmin y hy
              unfold Before at this hbefore ⊢; omega
            · simp only [List.mem_singleton] at hy
              subst hy
              unfold Before; omega
          · simp only [Bool.false_eq_true, ite_false, hbefore]
            refine ⟨by simp [hb], ?_⟩
            intro y hy
            rcases List.mem_append.mp hy with hy | hy
            · exact hmin y hy
            · simp only [List.mem_singleton] at hy
              subst hy
              exact hbefore
    have := ih (seen ++ [x]) _ key
    simpa using this

theorem plain_inv :
    ∀ (l seen : List α) (best : Option α), Inv cost idx seen best →
      (∀ y ∈ seen, ∀ z ∈ l, idx y < idx z) → l.Pairwise (fun a b => idx a < idx b) →
      Inv cost idx (seen ++ l) (l.foldl (plainStep cost) best) := by
  intro l
  induction l with
  | nil => intro seen best h _ _; simpa using h
  | cons x l ih =>
    intro seen best h hsep hpw
    obtain ⟨hx, hpw⟩ := List.pairwise_cons.mp hpw
    have key : Inv cost idx (seen ++ [x]) (plainStep cost best x) := by
      cases best with
      | none =>
        simp only [Inv] at h
        subst h
        refine ⟨by simp, ?_⟩
        intro y hy
        simp only [List.nil_append, List.mem_singleton] at hy
        subst hy
        unfold Before; omega
      | some b =>
        obtain ⟨hb, hmin⟩ := h
        have hidx := hsep b hb x List.mem_cons_self
        simp only [plainStep]
        by_cases hlt : cost x < cost b
        · simp only [hlt, ite_true]
          refine ⟨by simp, ?_⟩
          intro y hy
          rcases List.mem_append.mp hy with hy | hy
          · have := hmin y hy
            unfold Before at this ⊢; omega
          · simp only [List.mem_singleton] at hy
            subst hy
            unfold Before; omega
        · simp only [hlt, ite_false]
          refine ⟨by simp [hb], ?_⟩
          intro y hy
          rcases List.mem_append.mp hy with hy | hy
          · exact hmin y hy
          · simp only [List.mem_singleton] at hy
            subst hy
            unfold Before; omega
    have hsep2 : ∀ y ∈ seen ++ [x], ∀ z ∈ l, idx y < idx z := by
      intro y hy z hz
      rcases List.mem_append.mp hy with hy | hy
      · exact hsep y hy z (List.mem_cons_of_mem x hz)
      · simp only [List.mem_singleton] at hy
        subst hy
        exact hx z hz
    have := ih (seen ++ [x]) _ key hsep2 hpw
    simpa using this

/-- Options listed in order have one position each. -/
theorem idx_injective : ∀ (l : List α), l.Pairwise (fun a b => idx a < idx b) →
    ∀ x ∈ l, ∀ y ∈ l, idx x = idx y → x = y := by
  intro l
  induction l with
  | nil => intro _ x hx; cases hx
  | cons a l ih =>
    intro hpw x hx y hy heq
    obtain ⟨ha, hpw⟩ := List.pairwise_cons.mp hpw
    rcases List.mem_cons.mp hx with hxa | hxl <;> rcases List.mem_cons.mp hy with hya | hyl
    · rw [hxa, hya]
    · have := ha y hyl; rw [hxa] at heq; omega
    · have := ha x hxl; rw [hya] at heq; omega
    · exact ih hpw x hxl y hyl heq

/-- **The order of pricing changes no choice.** Pricing the options `ys`,
the listed `xs` in any order, skipping those priced strictly above the
best so far, chooses what `min_by` chooses over `xs`. -/
theorem anyOrder_eq (skip : Int → α → Bool) (sound : ∀ b x, skip (cost b) x = true → cost b < cost x)
    (xs ys : List α) (same : ∀ x, x ∈ ys ↔ x ∈ xs) (listed : xs.Pairwise (fun a b => idx a < idx b)) :
    ys.foldl (orderedStep cost idx skip) none = xs.foldl (plainStep cost) none := by
  have hy := ordered_inv cost idx skip sound ys [] none rfl
  have hx := plain_inv cost idx xs [] none rfl (by intro y hy; cases hy) listed
  simp only [List.nil_append] at hy hx
  cases hry : ys.foldl (orderedStep cost idx skip) none with
  | none =>
    rw [hry] at hy
    simp only [Inv] at hy
    cases hrx : xs.foldl (plainStep cost) none with
    | none => rfl
    | some b =>
      rw [hrx] at hx
      have := (same b).mpr hx.1
      rw [hy] at this; cases this
  | some b =>
    rw [hry] at hy
    cases hrx : xs.foldl (plainStep cost) none with
    | none =>
      rw [hrx] at hx
      simp only [Inv] at hx
      have := (same b).mp hy.1
      rw [hx] at this; cases this
    | some c =>
      rw [hrx] at hx
      obtain ⟨hb, hbmin⟩ := hy
      obtain ⟨hc, hcmin⟩ := hx
      have h1 := hbmin c ((same c).mpr hc)
      have h2 := hcmin b ((same b).mp hb)
      have : idx b = idx c := by unfold Before at h1 h2; omega
      rw [idx_injective idx xs listed b ((same b).mp hb) c hc this]

end Nicevg.AnyOrder
