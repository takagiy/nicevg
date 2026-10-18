// Apache Druid architecture, based on the official architecture, Kafka ingestion,
// query processing, storage, and Coordinator docs. Shows a Kafka record becoming
// a published segment, its handoff to Historicals, and the query path over those segments.
// https://druid.apache.org/docs/latest/design/architecture/
// https://druid.apache.org/docs/latest/ingestion/kafka-ingestion/
// https://druid.apache.org/docs/latest/ingestion/supervisor/
// https://druid.apache.org/docs/latest/querying/query-processing/
// https://druid.apache.org/docs/latest/design/storage/
// https://druid.apache.org/docs/latest/design/coordinator/

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

const nodes: Node[] = [
  { id: "master", class: "box", x: 360, y: 20, width: 560, height: 210, label: "Master server", fill: "#f1f5f9" },
  { id: "query", class: "box", x: 20, y: 260, width: 360, height: 410, label: "Query server", fill: "#eff6ff" },
  { id: "data", class: "box", x: 430, y: 290, width: 490, height: 510, label: "Data server", fill: "#f0fdf4" },
  { id: "kafka", class: "box", x: 20, y: 60, label: "Apache Kafka\n topic partitions + offsets", fill: "#fef3c7" },
  { id: "client", class: "box", x: 20, y: 735, label: "SQL / native query client" },
  { id: "router", class: "box", x: 65, y: 325, label: "Router\nAPI gateway + web console", parent: "query", fill: "#dbeafe" },
  { id: "broker", class: "box", x: 65, y: 475, label: "Broker\nsegment pruning + query fan-out", parent: "query", fill: "#bfdbfe" },
  { id: "coordinator", class: "box", x: 405, y: 75, label: "Coordinator\nsegment assignment + balancing", parent: "master", fill: "#e0e7ff" },
  { id: "overlord", class: "box", x: 690, y: 75, label: "Overlord\ntask scheduling + segment publishing", parent: "master", fill: "#e0e7ff" },
  { id: "supervisor", class: "box", x: 675, y: 345, label: "Kafka Supervisor\npartitions · offsets · replicas", parent: "data", fill: "#dcfce7" },
  { id: "task1", class: "box", x: 490, y: 470, label: "Kafka indexing task\nread / build segment", parent: "data", fill: "#bbf7d0" },
  { id: "task2", class: "box", x: 700, y: 470, label: "Kafka indexing task\nreplica / next task set", parent: "data", fill: "#bbf7d0" },
  { id: "mm", class: "box", x: 485, y: 390, label: "Middle Manager\nworker capacity + Peon lifecycle", parent: "data" },
  { id: "peon", class: "box", x: 500, y: 565, label: "Peon JVM\none task per process", parent: "data" },
  { id: "realtime", class: "box", x: 710, y: 580, label: "Realtime segment\nqueryable before publish", parent: "data", fill: "#fef9c3" },
  { id: "historical", class: "box", x: 490, y: 680, label: "Historical\nserves loaded immutable segments", parent: "data", fill: "#bbf7d0" },
  { id: "taskapi", class: "box", x: 470, y: 260, label: "Overlord task API" },
  { id: "deep", class: "box", x: 1030, y: 400, label: "Deep storage\nsegment files (S3 / HDFS / shared FS)", fill: "#fce7f3" },
  { id: "metadata", class: "box", x: 1030, y: 540, label: "Metadata store (PostgreSQL / MySQL)\nsegment + task + supervisor metadata", fill: "#fce7f3" },
  { id: "zk", class: "box", x: 1030, y: 680, label: "ZooKeeper\nservice discovery + leader election", fill: "#fce7f3" },
  { id: "spec", class: "box", x: 20, y: 125, label: "Supervisor spec\ndataSchema · ioConfig · tuningConfig" },
  { id: "parse", class: "box", x: 20, y: 200, label: "inputFormat + timestamp / dimensions / metrics" },
  { id: "segments", class: "box", x: 1030, y: 270, label: "Druid segments\ntime chunk · version · partition" },
  { id: "rules", class: "box", x: 1030, y: 190, label: "Coordinator load rules\ntier + replication factor" },
  { id: "queryreq", class: "box", x: 20, y: 555, label: "Druid SQL / native query" },
  { id: "subquery", class: "box", x: 250, y: 560, label: "Rewritten subqueries\nselected segment set" },
  { id: "partials", class: "box", x: 250, y: 635, label: "Partial results" },
  { id: "final", class: "box", x: 20, y: 635, label: "Merged result" },
  { id: "handoff", class: "box", x: 700, y: 695, label: "Handoff complete\nHistorical serving segment" },
  { id: "offsets", class: "box", x: 1030, y: 80, label: "Committed Kafka offsets\ntransactional with segment metadata" },
  { id: "announce", class: "box", x: 1030, y: 335, label: "Published segment metadata\nused flag + segment descriptor" },
  { id: "coordpoll", class: "box", x: 1030, y: 620, label: "Coordinator polls used segments" },
];

const edges = [
  { from: "client", to: "router", label: "HTTP query / API request" },
  { from: "router", to: "broker", label: "query endpoint" },
  { from: "router", to: "overlord", label: "task + supervisor API" },
  { from: "router", to: "coordinator", label: "cluster / segment API" },
  { from: "client", to: "queryreq", label: "SQL or native JSON" },
  { from: "queryreq", to: "broker", label: "query" },
  { from: "broker", to: "subquery", label: "prune by time / attributes; rewrite" },
  { from: "broker", to: "historical", label: "per-segment subquery" },
  { from: "broker", to: "realtime", label: "subquery for uncommitted data" },
  { from: "historical", to: "partials", label: "segment query results" },
  { from: "realtime", to: "partials", label: "segment query results" },
  { from: "partials", to: "broker", label: "partial results" },
  { from: "broker", to: "final", label: "merge partial results" },
  { from: "final", to: "client", label: "query response" },
  { from: "spec", to: "taskapi", label: "POST supervisor spec" },
  { from: "taskapi", to: "overlord", label: "HTTP task submission" },
  { from: "overlord", to: "supervisor", label: "spawn / restore supervisor" },
  { from: "supervisor", to: "kafka", label: "discover partitions; read offsets" },
  { from: "kafka", to: "task1", label: "records from assigned partitions" },
  { from: "kafka", to: "task2", label: "replica reads same stream offsets" },
  { from: "supervisor", to: "task1", label: "assign partitions + start offsets" },
  { from: "supervisor", to: "task2", label: "task status / replacement / retry" },
  { from: "task1", to: "mm", label: "task request" },
  { from: "mm", to: "peon", label: "spawn task JVM" },
  { from: "peon", to: "task1", label: "runs indexing task" },
  { from: "parse", to: "task1", label: "parse and transform records" },
  { from: "task1", to: "realtime", label: "incremental segment; queryable now" },
  { from: "task1", to: "deep", label: "push completed segment file" },
  { from: "task1", to: "metadata", label: "publish segment + Kafka offsets atomically" },
  { from: "task1", to: "announce", label: "segment descriptor / used=true" },
  { from: "metadata", to: "offsets", label: "persist checkpoint with publish" },
  { from: "metadata", to: "announce", label: "segment metadata record" },
  { from: "supervisor", to: "metadata", label: "persist spec + checkpoints; recover on leader change" },
  { from: "overlord", to: "metadata", label: "task state + supervisor specs" },
  { from: "overlord", to: "mm", label: "assign task to worker capacity" },
  { from: "coordinator", to: "metadata", label: "poll used segments + load rules" },
  { from: "rules", to: "coordinator", label: "tier / replication policy" },
  { from: "announce", to: "segments", label: "published segment identity" },
  { from: "deep", to: "historical", label: "download segment file" },
  { from: "coordinator", to: "historical", label: "HTTP load / drop / balance" },
  { from: "historical", to: "coordpoll", label: "HTTP served-segment inventory" },
  { from: "coordpoll", to: "coordinator", label: "assess cluster state each run" },
  { from: "historical", to: "handoff", label: "segment loaded and serving" },
  { from: "handoff", to: "task1", label: "task exits after realtime handoff" },
  { from: "deep", to: "segments", label: "durable segment bytes" },
  { from: "metadata", to: "segments", label: "schema, size, deep-storage location" },
  { from: "coordinator", to: "zk", label: "leader election + service discovery" },
  { from: "overlord", to: "zk", label: "leader election + worker discovery" },
  { from: "broker", to: "zk", label: "service discovery" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
