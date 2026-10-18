// NATS JetStream cluster internals, using the official `east` cluster and
// `ORDERS` example. Shows route gossip, independent meta and stream RAFT groups,
// the quorum-backed write path, and leader election after a server loss.
// https://docs.nats.io/learn/clustering/forming-a-cluster
// https://docs.nats.io/learn/clustering/raft-and-leaders
// https://docs.nats.io/learn/clustering/replication-and-r3
// https://docs.nats.io/learn/clustering/scaling-and-peers

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

const nodes: Node[] = [
  { id: "east", class: "box", x: 300, y: 20, width: 800, height: 850, label: "NATS cluster `east` · JetStream enabled", fill: "#f8fafc" },
  { id: "order-svc", class: "box", x: 20, y: 130, label: "order-svc\nJetStream publish" },
  { id: "pub", class: "box", x: 20, y: 260, label: "orders.created\norder_id · customer · total_cents · ts" },
  { id: "puback", class: "box", x: 20, y: 400, label: "PubAck\nstream=ORDERS · sequence" },
  { id: "n1", class: "box", x: 380, y: 110, width: 200, label: "n1-east\nclient :4222 · route :6222", parent: "east", fill: "#dbeafe" },
  { id: "n2", class: "box", x: 640, y: 110, width: 200, label: "n2-east\nclient :4223 · route :6223", parent: "east", fill: "#dbeafe" },
  { id: "n3", class: "box", x: 880, y: 110, width: 200, label: "n3-east\nclient :4224 · route :6224", parent: "east", fill: "#dbeafe" },
  { id: "n1-js", class: "box", x: 380, y: 245, width: 200, label: "JetStream on n1\n./js/n1-east", parent: "east", fill: "#dcfce7" },
  { id: "n2-js", class: "box", x: 640, y: 245, width: 200, label: "JetStream on n2\n./js/n2-east", parent: "east", fill: "#dcfce7" },
  { id: "n3-js", class: "box", x: 880, y: 245, width: 200, label: "JetStream on n3\n./js/n3-east", parent: "east", fill: "#dcfce7" },
  { id: "route12", class: "box", x: 500, y: 175, label: "bidirectional route" , parent: "east" },
  { id: "route13", class: "box", x: 790, y: 175, label: "bidirectional route", parent: "east" },
  { id: "route23", class: "box", x: 740, y: 205, label: "implicit route", parent: "east" },
  { id: "info", class: "box", x: 385, y: 330, label: "INFO gossip\npeer address forwarded on routes", parent: "east" },
  { id: "meta", class: "box", x: 390, y: 440, width: 660, height: 95, label: "Meta RAFT group · all 3 servers\nassignment log: streams · replica count · peer placement", parent: "east", fill: "#ede9fe" },
  { id: "meta-leader", class: "box", x: 575, y: 555, label: "Meta leader\nassigns ORDERS replicas", parent: "east", fill: "#ddd6fe" },
  { id: "stream", class: "box", x: 390, y: 625, width: 660, height: 90, label: "ORDERS stream RAFT group · R=3 · subjects `orders.>`", parent: "east", fill: "#fef3c7" },
  { id: "stream-leader", class: "box", x: 390, y: 745, width: 200, label: "Stream leader\naccepts stream writes", parent: "east", fill: "#fde68a" },
  { id: "follower2", class: "box", x: 640, y: 745, width: 200, label: "Follower n2\nreplica log + store", parent: "east" },
  { id: "follower3", class: "box", x: 880, y: 745, width: 200, label: "Follower n3\nreplica log + store", parent: "east" },
  { id: "append", class: "box", x: 1120, y: 270, label: "Append entry to\nleader log" },
  { id: "replicate", class: "box", x: 1120, y: 390, label: "Replicate entry\nto followers" },
  { id: "quorum", class: "box", x: 1120, y: 510, label: "Quorum commit\nleader + 1 of 2" },
  { id: "apply", class: "box", x: 1120, y: 630, label: "Apply committed entry\nstream sequence advances" },
  { id: "heartbeat", class: "box", x: 1120, y: 760, label: "Leader heartbeat\n~1 second" },
  { id: "down", class: "box", x: 20, y: 580, label: "Leader server unavailable\nheartbeat stops" },
  { id: "timer", class: "box", x: 20, y: 690, label: "Follower election timer\n4–9 seconds" },
  { id: "candidate", class: "box", x: 20, y: 800, label: "Candidate increments term\nand requests peer votes" },
  { id: "votes", class: "box", x: 300, y: 920, label: "Quorum of votes\n2 of 3; up-to-date log required" },
  { id: "newleader", class: "box", x: 650, y: 920, label: "New stream leader\nresumes writes + heartbeats" },
  { id: "retry", class: "box", x: 1020, y: 920, label: "order-svc retries\nbrief no-leader window" },
  { id: "n4", class: "box", x: 1360, y: 120, label: "n4-east joins\nroute mesh first" },
  { id: "resize", class: "box", x: 1360, y: 250, label: "stream edit\n--replicas=4" },
  { id: "assignment", class: "box", x: 1360, y: 380, label: "Meta leader selects\nplacement-eligible peer" },
  { id: "catchup", class: "box", x: 1360, y: 510, label: "New peer catches up\nleader streams missing log entries" },
  { id: "current", class: "box", x: 1360, y: 640, label: "Replica current\nlag reaches zero" },
];

const edges = [
  { from: "order-svc", to: "pub", label: "subject + payload" },
  { from: "pub", to: "n1", label: "client protocol :4222" },
  { from: "n1", to: "stream-leader", label: "route to current leader" },
  { from: "stream-leader", to: "append", label: "ordered log entry" },
  { from: "append", to: "replicate", label: "leader sends entry" },
  { from: "replicate", to: "follower2", label: "route replication" },
  { from: "replicate", to: "follower3", label: "route replication" },
  { from: "follower2", to: "quorum", label: "entry stored" },
  { from: "stream-leader", to: "quorum", label: "leader copy stored" },
  { from: "quorum", to: "apply", label: "majority confirms" },
  { from: "apply", to: "stream", label: "commit + apply" },
  { from: "apply", to: "puback", label: "only after quorum" },
  { from: "puback", to: "order-svc", label: "success response" },
  { from: "heartbeat", to: "follower2", label: "stream group" },
  { from: "heartbeat", to: "follower3", label: "stream group" },
  { from: "down", to: "timer", label: "no heartbeat" },
  { from: "timer", to: "candidate", label: "timer fires" },
  { from: "candidate", to: "follower2", label: "request vote" },
  { from: "candidate", to: "follower3", label: "request vote" },
  { from: "follower2", to: "votes", label: "grant if eligible" },
  { from: "candidate", to: "votes", label: "self vote" },
  { from: "votes", to: "newleader", label: "majority elects" },
  { from: "newleader", to: "stream-leader", label: "leadership transfers" },
  { from: "newleader", to: "retry", label: "writes available" },
  { from: "retry", to: "order-svc", label: "retry publish" },
  { from: "n1", to: "route12", label: "explicit seed route" },
  { from: "n2", to: "route12" },
  { from: "n1", to: "route13", label: "explicit seed route" },
  { from: "n3", to: "route13" },
  { from: "info", to: "route23", label: "peer address via INFO" },
  { from: "n2", to: "route23" },
  { from: "n3", to: "route23" },
  { from: "n1", to: "info" },
  { from: "n2", to: "info" },
  { from: "n3", to: "info" },
  { from: "meta", to: "meta-leader", label: "independent RAFT election" },
  { from: "meta-leader", to: "stream", label: "assignment + R=3 placement" },
  { from: "n1-js", to: "meta", label: "meta peer" },
  { from: "n2-js", to: "meta", label: "meta peer" },
  { from: "n3-js", to: "meta", label: "meta peer" },
  { from: "n1-js", to: "stream", label: "stream peer" },
  { from: "n2-js", to: "stream", label: "stream peer" },
  { from: "n3-js", to: "stream", label: "stream peer" },
  { from: "n4", to: "resize", label: "running cluster member" },
  { from: "resize", to: "assignment", label: "replica count request" },
  { from: "meta-leader", to: "assignment", label: "records new peer set" },
  { from: "assignment", to: "catchup", label: "new stream peer" },
  { from: "stream-leader", to: "catchup", label: "missing log entries" },
  { from: "catchup", to: "current", label: "lag shrinks to zero" },
  { from: "current", to: "stream", label: "eligible data replica" },
];

const sketch = {
  nodes,
  edges,
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
