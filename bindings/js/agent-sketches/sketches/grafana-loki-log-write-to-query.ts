import type { Sketch } from "nicevg";
import { svg, text } from "nicevg";

// Grafana Loki — https://grafana.com/docs/loki/latest/get-started/architecture/
// https://github.com/grafana/loki/blob/main/pkg/distributor/distributor.go
// https://github.com/grafana/loki/blob/main/pkg/ingester/ingester.go
// https://github.com/grafana/loki/blob/main/pkg/querier/querier.go
// Follows labeled log lines from a Promtail client through replication, WAL and chunk storage,
// then follows a LogQL query back through ingesters and object storage to Grafana.

type Node = { id: string; class: "box" | "store" | "process" | "outside"; x: number; y: number; label: string; fill?: string; parent?: string; width?: number; height?: number };

const box = ({ label, fill = "#eff6ff", width = 174, height = 64 }: Node) => {
  const t = text(label, 12);
  return svg`<rect width="${Math.max(width, t.width + 20)}" height="${Math.max(height, t.height + 18)}" rx="5" fill="${fill}" stroke="#334155"/><path d="M0 9h${Math.max(width, t.width + 20)}" stroke="#334155"/><rect width="8" height="8" fill="#60a5fa"/>${t.at({ cx: Math.max(width, t.width + 20) / 2, cy: Math.max(height, t.height + 18) / 2 + 5 })}`;
};
const store = ({ label, fill = "#fef3c7", width = 174, height = 70 }: Node) => {
  const t = text(label, 12);
  return svg`<path d="M0 12 Q${width / 2} -5 ${width} 12 V${height - 12} Q${width / 2} ${height + 5} 0 ${height - 12}Z" fill="${fill}" stroke="#92400e"/><ellipse cx="${width / 2}" cy="12" rx="${width / 2}" ry="12" fill="${fill}" stroke="#92400e"/>${t.at({ cx: width / 2, cy: height / 2 + 5 })}`;
};
const process = ({ label, fill = "#dcfce7", width = 174, height = 68 }: Node) => {
  const t = text(label, 12);
  return svg`<polygon points="14,0 ${width},0 ${width - 14},${height} 0,${height}" fill="${fill}" stroke="#166534"/>${t.at({ cx: width / 2, cy: height / 2 })}`;
};
const outside = ({ label, fill = "#fce7f3", width = 174, height = 68 }: Node) => {
  const t = text(label, 12);
  return svg`<circle cx="${width / 2}" cy="${height / 2}" r="${Math.min(width, height) / 2}" fill="${fill}" stroke="#9d174d"/>${t.at({ cx: width / 2, cy: height / 2 })}`;
};

const nodes: Node[] = [
  { id: "tail", class: "outside", x: 16, y: 70, label: "Application log files\n/var/log/app/*.log" },
  { id: "positions", class: "store", x: 22, y: 178, label: "Promtail positions.yaml\nfile offsets" },
  { id: "scrape", class: "process", x: 238, y: 62, label: "Promtail scrape_configs\npath + __path__ labels" },
  { id: "tailer", class: "process", x: 240, y: 164, label: "file target / tailer\nread new lines" },
  { id: "pipeline", class: "box", x: 430, y: 74, label: "Promtail pipeline_stages\nparse → timestamp → labels" },
  { id: "relabel", class: "process", x: 442, y: 184, label: "relabel_configs\nkeep / drop / rewrite labels" },
  { id: "batch", class: "box", x: 644, y: 82, label: "client batch\nstream labels + entries" },
  { id: "clientretry", class: "process", x: 646, y: 190, label: "client retry queue\nretry failed push" },
  { id: "push", class: "outside", x: 850, y: 94, label: "POST /loki/api/v1/push\nSnappy protobuf" },
  { id: "gateway", class: "box", x: 1050, y: 24, label: "Loki gateway / reverse proxy\nroute write and query APIs" },
  { id: "tenant", class: "process", x: 1052, y: 138, label: "HTTP middleware\nX-Scope-OrgID tenant" },
  { id: "distributor", class: "box", x: 1244, y: 74, label: "Distributor\nvalidate labels / limits" },
  { id: "hash", class: "process", x: 1244, y: 178, label: "hash stream labels\nselect replication set" },
  { id: "ring", class: "store", x: 1450, y: 178, label: "Ingester ring\nhealthy tokens" },
  { id: "ing_a", class: "box", x: 1450, y: 26, label: "Ingester A\nreplica 1" },
  { id: "ing_b", class: "box", x: 1650, y: 92, label: "Ingester B\nreplica 2" },
  { id: "ing_c", class: "box", x: 1650, y: 202, label: "Ingester C\nreplica 3" },
  { id: "stream", class: "process", x: 1450, y: 292, label: "per-tenant stream\nlabel set + chunk builder" },
  { id: "head", class: "store", x: 1648, y: 310, label: "in-memory head chunk\ncompressed log entries" },
  { id: "wal", class: "store", x: 1450, y: 400, label: "WAL\nrecords before ack" },
  { id: "cut", class: "process", x: 1646, y: 420, label: "cut chunk\nsize / age / idle" },
  { id: "flush", class: "process", x: 1646, y: 522, label: "flush op\nencode + upload" },
  { id: "index", class: "store", x: 1446, y: 538, label: "TSDB index\nseries → chunk refs" },
  { id: "chunks", class: "store", x: 1648, y: 624, label: "Object store\nchunk bytes (S3/GCS)" },
  { id: "ack", class: "process", x: 1244, y: 302, label: "replication quorum\nwait for successful appends" },
  { id: "ackback", class: "outside", x: 1050, y: 310, label: "HTTP 204\npush accepted" },
  { id: "grafana", class: "outside", x: 18, y: 430, label: "Grafana Explore\nLogQL + time range" },
  { id: "queryapi", class: "outside", x: 240, y: 420, label: "GET /loki/api/v1/query_range\nquery + start/end/limit" },
  { id: "qgateway", class: "box", x: 438, y: 422, label: "gateway query route\nforward HTTP request" },
  { id: "qfront", class: "box", x: 644, y: 412, label: "Query Frontend\nvalidate / split interval" },
  { id: "split", class: "process", x: 848, y: 406, label: "query range sharding\nparallel subqueries" },
  { id: "scheduler", class: "store", x: 850, y: 514, label: "query scheduler\nqueued requests" },
  { id: "worker", class: "process", x: 1050, y: 424, label: "querier frontend_worker\npick queued query" },
  { id: "querier", class: "box", x: 1246, y: 422, label: "Querier\nLogQL engine" },
  { id: "parse", class: "process", x: 1246, y: 524, label: "parse LogQL\nselector + pipeline" },
  { id: "selector", class: "process", x: 1450, y: 722, label: "match stream labels\n{app=\"api\", env=\"prod\"}" },
  { id: "queryring", class: "store", x: 1050, y: 632, label: "Ingester ring\nfind live replicas" },
  { id: "live", class: "process", x: 1246, y: 632, label: "query in-memory data\nrecent head chunks" },
  { id: "stored", class: "process", x: 1450, y: 632, label: "store gateway / TSDB\nfind chunk references" },
  { id: "download", class: "process", x: 1648, y: 730, label: "fetch + decompress\nselected chunks" },
  { id: "filter", class: "process", x: 1648, y: 820, label: "line filters / parsers\n|=, json, logfmt" },
  { id: "merge", class: "process", x: 1450, y: 850, label: "merge sorted streams\ntime order + limit" },
  { id: "response", class: "box", x: 1246, y: 852, label: "query response\nstreams, labels, values" },
  { id: "json", class: "process", x: 1050, y: 852, label: "encode Loki JSON\nresultType = streams" },
  { id: "render", class: "outside", x: 848, y: 854, label: "Grafana log rows\nlabels + timestamps + lines" },
];

const edges = [
  { from: "tail", to: "tailer", label: "bytes appended" }, { from: "scrape", to: "tailer", label: "target config" },
  { from: "tailer", to: "positions", label: "persist read offset" }, { from: "tailer", to: "pipeline", label: "line + filename labels" },
  { from: "pipeline", to: "relabel", label: "parsed entry" }, { from: "relabel", to: "batch", label: "final stream labels" },
  { from: "batch", to: "push", label: "batched entries" }, { from: "clientretry", to: "push", label: "retry request" },
  { from: "push", to: "gateway", label: "write HTTP" }, { from: "gateway", to: "tenant", label: "forward request" },
  { from: "tenant", to: "distributor", label: "tenant + protobuf" }, { from: "distributor", to: "hash", label: "validated streams" },
  { from: "hash", to: "ring", label: "lookup tokens" }, { from: "hash", to: "ing_a", label: "replica append" },
  { from: "hash", to: "ing_b", label: "replica append" }, { from: "hash", to: "ing_c", label: "replica append" },
  { from: "ing_a", to: "stream" }, { from: "ing_b", to: "stream" }, { from: "ing_c", to: "stream" },
  { from: "stream", to: "wal", label: "append before acknowledge" }, { from: "stream", to: "head", label: "append timestamp + line" },
  { from: "head", to: "cut", label: "chunk full / idle / old" }, { from: "cut", to: "flush", label: "closed chunk" },
  { from: "flush", to: "chunks", label: "chunk object" }, { from: "flush", to: "index", label: "series + chunk metadata" },
  { from: "ing_a", to: "ack", label: "append result" }, { from: "ing_b", to: "ack", label: "append result" },
  { from: "ing_c", to: "ack", label: "append result" }, { from: "ack", to: "ackback", label: "quorum success" },
  { from: "ackback", to: "clientretry", label: "non-2xx retries" }, { from: "grafana", to: "queryapi", label: "query expression" },
  { from: "queryapi", to: "qgateway" }, { from: "qgateway", to: "qfront" }, { from: "qfront", to: "split", label: "time-split requests" },
  { from: "split", to: "scheduler", label: "enqueue subqueries" }, { from: "scheduler", to: "worker", label: "dequeue" },
  { from: "worker", to: "querier", label: "execute request" }, { from: "querier", to: "parse", label: "LogQL AST" },
  { from: "parse", to: "selector", label: "matchers + time bounds" }, { from: "selector", to: "live", label: "recent matching streams" },
  { from: "queryring", to: "live", label: "replica addresses" }, { from: "live", to: "merge", label: "head entries" },
  { from: "selector", to: "stored", label: "series IDs / time range" }, { from: "stored", to: "index", label: "lookup index" },
  { from: "stored", to: "download", label: "chunk refs" }, { from: "download", to: "chunks", label: "get object" },
  { from: "download", to: "filter", label: "decoded entries" }, { from: "filter", to: "merge", label: "matching log lines" },
  { from: "merge", to: "response", label: "ordered streams" }, { from: "response", to: "json" },
  { from: "json", to: "render", label: "HTTP JSON response" },
];

export default { nodes, edges, style: { box, store, process, outside }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
