import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

// Rust governor — https://docs.rs/governor/latest/governor/state/struct.RateLimiter.html
// https://docs.rs/governor/latest/src/governor/middleware.rs.html
// https://docs.rs/governor/latest/governor/struct.Quota.html
// Shows a keyed check_key decision: the key selects its GCRA state, whose theoretical
// arrival time is atomically compared with the quota and current clock to allow or reject a cell.

type Node = SketchNode & { class: "box"; label: string; fill?: string };

export default {
  nodes: [
    { id: "check", class: "box", x: 40, y: 170, label: "check_key(&key)\none cell", fill: "#dbeafe" },
    { id: "key", class: "box", x: 245, y: 40, label: "K: Hash + Eq\ncustomer / client key", fill: "#e0e7ff" },
    { id: "store", class: "box", x: 500, y: 100, label: "DefaultKeyedStateStore\nDashMap<K, InMemoryState>\nmissing key → fresh state", fill: "#dcfce7" },
    { id: "state", class: "box", x: 800, y: 40, label: "InMemoryState\nAtomicU64: theoretical\narrival time (TAT)", fill: "#fae8ff" },
    { id: "quota", class: "box", x: 480, y: 330, label: "Quota\nreplenish interval: T\nburst tolerance: τ", fill: "#fef3c7" },
    { id: "gcra", class: "box", x: 1060, y: 210, label: "Gcra.test_and_update()\nt0 < TAT − τ → reject\nelse next = max(TAT, t0) + T", fill: "#ffedd5" },
    { id: "outcome", class: "box", x: 1360, y: 120, label: "Ok(positive)\ncell admitted", fill: "#dcfce7" },
    { id: "denied", class: "box", x: 1370, y: 330, label: "Err(NotUntil)\nearlyest_possible\nwait_time_from(now)", fill: "#fee2e2" },
  ],
  edges: [
    { from: "check", to: "key", label: "lookup key" },
    { from: "key", to: "store", label: "hash + equality lookup" },
    { from: "store", to: "state", label: "selected per-key state" },
    { from: "state", to: "gcra", label: "load TAT; CAS update" },
    { from: "quota", to: "gcra", label: "T and τ from quota" },
    { from: "gcra", to: "outcome", label: "condition passes; reserve cell" },
    { from: "gcra", to: "denied", label: "TAT too far ahead" },
    { from: "denied", to: "check", label: "caller retries after wait" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
