/-!
# A change beside every node leaves the nodes as read

Fixing reads a draft's whole diagram again after each change
(`Draft::report`, src/fix.rs). Reading the nodes is most of that on
diagrams with many texts: every node's shape, its texts measured, its
ports. Writing a connector's new route replaces its shaft and arrowheads,
and moving a label rewrites its texts; where those stand beside every
node's group (`ReadNodes::stand_beside`: none is a group, lies inside one
or holds one), the nodes read before are kept for the changed draft
(`changed_beside_nodes`).

Arranging also tries moving nodes: a candidate translates the groups of
the nodes it moves and reads the document again. Only those groups'
attributes change, so every group beside them is the node it was, and is
taken from the nodes read for the state the candidate starts from
(`Draft::with_groups_moved`, `read_nodes`); the moved groups, what they
hold and what holds them are read again.

`reads_same` is why they are the same. A node is read from what stands at
and below its group (its shape, texts and ports) and from the elements
above it (the properties and translations it inherits). Replacing what
stands at a path beside the group changes neither.

Checked in the Rust code (2026-11-19): `read_nodes` (src/diagram.rs) reads
a node from its group's descendants and lineage only, but for two things
that depend on which elements are groups: a group without an id is named
by its position among all groups, and a group inside a custom node's
picture is left out. The elements replaced are a shaft (a line, polyline
or path, replaced by a path), arrowheads (polygons) and texts with their
spans: none is a group or holds one, before or after, so the groups stay
where they are.
-/

namespace Nicevg.Beside

variable {E : Type}

/-- A document: the element at each path, if there is one. -/
abbrev Doc (E : Type) := List Nat → Option E

/-- The document with what stands at `p` and below it replaced by `sub`. -/
def replace (doc : Doc E) (p : List Nat) (sub : Doc E) : Doc E :=
  fun r => if p.isPrefixOf r then sub (r.drop p.length) else doc r

/-- Two paths stand beside each other: neither holds the other. -/
def Beside (p q : List Nat) : Prop := ¬p <+: q ∧ ¬q <+: p

/-- What stands at a path's ancestors and at the path itself is unchanged. -/
theorem above_unchanged (doc sub : Doc E) (p q : List Nat) (h : Beside p q) (r : List Nat)
    (hr : r <+: q) : replace doc p sub r = doc r := by
  unfold replace
  have : ¬p <+: r := fun hp => h.1 (List.IsPrefix.trans hp hr)
  rw [ite_eq_right (by simpa [List.isPrefixOf_iff_prefix] using this)]

/-- What stands at a path and below it is unchanged. -/
theorem below_unchanged (doc sub : Doc E) (p q : List Nat) (h : Beside p q) (s : List Nat) :
    replace doc p sub (q ++ s) = doc (q ++ s) := by
  unfold replace
  have : ¬p <+: q ++ s := by
    intro hp
    rcases List.prefix_or_prefix_of_prefix hp (List.prefix_append q s) with h1 | h2
    · exact h.1 h1
    · exact h.2 h2
  rw [ite_eq_right (by simpa [List.isPrefixOf_iff_prefix] using this)]

/-- **Reading a node gives the same after a change beside it.** Whatever a
node's reading makes of the elements at and below its group and of those
above it, it makes the same of the changed document. -/
theorem reads_same {R : Type} (read : (List Nat → Option E) → (Nat → Option E) → R)
    (doc sub : Doc E) (p q : List Nat) (h : Beside p q) :
    read (fun s => replace doc p sub (q ++ s)) (fun k => replace doc p sub (q.take k))
      = read (fun s => doc (q ++ s)) (fun k => doc (q.take k)) := by
  have below : (fun s => replace doc p sub (q ++ s)) = (fun s => doc (q ++ s)) := by
    funext s; exact below_unchanged doc sub p q h s
  have above : (fun k => replace doc p sub (q.take k)) = (fun k => doc (q.take k)) := by
    funext k; exact above_unchanged doc sub p q h _ (List.take_prefix k q)
  rw [below, above]

end Nicevg.Beside
