/-!
# Ends pinned to ports read no chosen side

`reroute_around` (src/fix.rs) first chooses a pair of node sides for every
edge it reroutes (`choose_all`, two route searches an edge and
more on its second pass), then pins each end that names a port to one of
the port's places. A chosen side is read in three places only:

* pricing a place: an end with no port starts from the middle of its
  chosen side, a pinned end from its place (`ports_of`);
* the sides the ports are then spread on: a pinned end meets the side its
  place lies on, another its chosen side;
* choosing sides for the other edges, which crowd away from the sides
  already taken.

`pinned_reads_no_side` proves the first two give the same whatever sides
were chosen, for an edge whose ends both name ports: every place
priced and the sides met come out equal. The third is read only while
sides are being chosen. So when every edge rerouted has both ends on
ports, nothing reads a chosen side, and the code does not search for any.

Checked in the Rust code (2026-11-19): an end whose port has a place is
always pinned (`Reroute::new` pins it to the nearest place, and the choice
among places keeps it on one), and `chosen` is read nowhere else.
-/

namespace Nicevg.Pinned

variable {Side Pin Point : Type}

/-- The port an end takes: its pin's point, or the middle of the side
chosen for it when it names no port. -/
def endPort (point : Pin → Point) (middle : Side → Point) (side : Side) : Option Pin → Point
  | some pin => point pin
  | none => middle side

/-- The side an end meets: its pin's, or the one chosen for it. -/
def endSide (sideOf : Pin → Side) (side : Side) : Option Pin → Side
  | some pin => sideOf pin
  | none => side

theorem endPort_pinned (point : Pin → Point) (middle : Side → Point) (s t : Side) (pin : Option Pin)
    (h : pin.isSome) : endPort point middle s pin = endPort point middle t pin := by
  cases pin with
  | none => cases h
  | some p => rfl

theorem endSide_pinned (sideOf : Pin → Side) (s t : Side) (pin : Option Pin) (h : pin.isSome) :
    endSide sideOf s pin = endSide sideOf t pin := by
  cases pin with
  | none => cases h
  | some p => rfl

/-- **An edge pinned at both ends reads no chosen side.** Whatever
sides `s` and `t` were chosen for it, every option of places is priced from
the same ports and meets the same sides. -/
theorem pinned_reads_no_side (point : Pin → Point) (sideOf : Pin → Side)
    (middleStart middleEnd : Side → Point) (options : List (Option Pin × Option Pin))
    (pinned : ∀ o ∈ options, o.1.isSome ∧ o.2.isSome) (s t : Side × Side) :
    options.map (fun o => (endPort point middleStart s.1 o.1, endPort point middleEnd s.2 o.2))
        = options.map (fun o => (endPort point middleStart t.1 o.1, endPort point middleEnd t.2 o.2))
      ∧ options.map (fun o => (endSide sideOf s.1 o.1, endSide sideOf s.2 o.2))
        = options.map (fun o => (endSide sideOf t.1 o.1, endSide sideOf t.2 o.2)) := by
  constructor
  · apply List.map_congr_left
    intro o ho
    obtain ⟨h1, h2⟩ := pinned o ho
    rw [endPort_pinned point middleStart s.1 t.1 o.1 h1, endPort_pinned point middleEnd s.2 t.2 o.2 h2]
  · apply List.map_congr_left
    intro o ho
    obtain ⟨h1, h2⟩ := pinned o ho
    rw [endSide_pinned sideOf s.1 t.1 o.1 h1, endSide_pinned sideOf s.2 t.2 o.2 h2]

end Nicevg.Pinned
