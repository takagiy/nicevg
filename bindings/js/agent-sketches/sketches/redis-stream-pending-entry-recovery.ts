// Redis Streams consumer-group pending-entry recovery, based on:
// https://redis.io/docs/latest/develop/data-types/streams/
// https://redis.io/docs/latest/commands/xreadgroup/
// https://redis.io/docs/latest/commands/xpending/
// https://redis.io/docs/latest/commands/xautoclaim/
// https://redis.io/docs/latest/commands/xack/
// The diagram follows one stream entry from append through delivery, a crashed consumer,
// pending inspection, idle-time claim, retry, and acknowledgement; the surrounding nodes
// name the stream, group, PEL, and commands that carry each transition.
import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = SketchNode & { class: "box"; label: string };

const labels = [
  "XADD",
  "Stream key\nrace:italy",
  "Stream radix tree\nrax entries",
  "Stream entry\n1692632647899-0",
  "Entry field\nrider",
  "Entry value\nRoyce",
  "XGROUP CREATE",
  "Consumer group\nitaly_riders",
  "last-delivered-id",
  "Consumer\nBob",
  "XREADGROUP",
  "STREAMS race:italy\n> (new entries)",
  "Delivery to Bob",
  "Group PEL\nentry reference",
  "Bob PEL\nentry reference",
  "delivery counter\n1",
  "last delivery time",
  "Consumer process\nBob",
  "Application handler\nprocess rider event",
  "Business side effect\nnot yet committed",
  "Consumer crash\nbefore XACK",
  "Connection closes",
  "Entry remains\nin stream",
  "Entry remains pending\nnot acknowledged",
  "Pending entry assigned\nto Bob",
  "Group work stalls\nfor this entry",
  "Recovery consumer\nAlice",
  "XPENDING summary",
  "XPENDING range\n- + COUNT 10",
  "Minimum idle time\nfilter",
  "Pending entry ID",
  "Current owner\nBob",
  "Idle milliseconds",
  "Times delivered",
  "Inspect entry body\nXRANGE ID ID",
  "Recovery policy\nidle threshold",
  "Threshold not met\nleave pending",
  "Wait; entry idle\ntime increases",
  "Threshold met\neligible to claim",
  "XAUTOCLAIM",
  "XCLAIM (explicit IDs)",
  "Start ID + COUNT\nscan cursor",
  "Claimed entry\nreturned to Alice",
  "Owner changes\nBob → Alice",
  "Idle time reset",
  "delivery counter\nincrements",
  "Alice PEL\nentry reference",
  "Bob PEL reference\nremoved",
  "Group PEL reference\nretained",
  "Alice handler\nretry processing",
  "Handler succeeds",
  "Duplicate side effect\npossible after crash",
  "Application idempotency\nby entry ID",
  "XACK",
  "Group PEL reference\nremoved",
  "Alice PEL reference\nremoved",
  "Entry no longer pending\nfor this group",
  "Stream entry\nstill retained",
  "XACK returns\n1 acknowledged",
  "XAUTOCLAIM next ID\ncontinue scan",
  "No idle entry\nfound in scan",
  "Retry delivery\nfrom PEL history",
  "XREADGROUP with ID\nreads own history",
  "XINFO GROUPS\nobserve group",
  "XINFO CONSUMERS\nobserve consumers",
  "AOF / RDB\npersist group state",
  "Replica propagation\nmay be asynchronous",
  "WAIT replicas\nrequest propagation",
];

const nodes = labels.map((label, i) => ({
  id: `n${i + 1}`,
  class: "box" as const,
  x: 30 + (i % 8) * 240,
  y: 30 + Math.floor(i / 8) * 115,
  label,
})) satisfies Node[];

const edges = [
  [1, 2, "append entry"], [2, 3, "stores"], [3, 4, "contains"], [4, 5, "field"], [4, 6, "value"],
  [7, 8, "create group"], [8, 9, "cursor"], [8, 10, "member"], [11, 12, "ID >"], [12, 13, "deliver"],
  [13, 4, "entry"], [13, 14, "record pending"], [14, 15, "per-consumer PEL"], [13, 16, "increment"], [13, 17, "set"],
  [13, 18, "to consumer"], [18, 19, "invoke"], [19, 20, "attempt"], [18, 21, "dies before ack"], [21, 22, "disconnect"],
  [21, 23, "not deleted"], [21, 24, "unacked reference remains"], [24, 25, "owner"], [25, 26, "no progress"],
  [27, 28, "inspect"], [28, 29, "inspect details"], [29, 30, "filter by age"], [29, 31, "reports"], [29, 32, "reports"],
  [29, 33, "reports"], [29, 34, "reports"], [31, 35, "retrieve body"], [35, 4, "same ID"], [33, 36, "compare"],
  [36, 37, "too young"], [37, 38, "wait"], [38, 33, "age grows"], [36, 39, "old enough"], [39, 40, "scan and claim"],
  [39, 41, "claim specific IDs"], [40, 42, "scan cursor"], [40, 43, "claim result"], [41, 43, "claim result"],
  [43, 44, "ownership changes"], [44, 45, "reset"], [44, 46, "increment"], [44, 47, "move reference"],
  [47, 48, "remove old owner"], [47, 49, "global reference stays"], [43, 50, "retry"], [50, 51, "successful attempt"],
  [51, 54, "acknowledge"], [50, 52, "crash may repeat effect"], [52, 53, "deduplicate by ID"],
  [54, 55, "remove"], [54, 56, "remove"], [54, 57, "settled"], [54, 58, "not deleted"], [54, 59, "count"],
  [42, 60, "continue scan"], [40, 61, "cursor / empty result"], [27, 62, "same owner can replay"],
  [62, 63, "ID other than >"], [8, 64, "observe"], [8, 65, "observe"], [8, 66, "persisted state"],
  [66, 67, "replicate"], [67, 68, "request propagation"],
].map(([from, to, label]) => ({ from: `n${from}`, to: `n${to}`, label: String(label) })) satisfies NonNullable<Sketch<Node>["edges"]>;

export default {
  nodes,
  edges,
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
