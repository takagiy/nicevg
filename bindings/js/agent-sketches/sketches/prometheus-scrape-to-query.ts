// Prometheus scrape-to-query data path.
// Sources: https://github.com/prometheus/prometheus/blob/main/documentation/internal_architecture.md
// https://prometheus.io/docs/prometheus/latest/storage/ and https://prometheus.io/docs/prometheus/latest/querying/api/
// Shows an exported metric becoming labeled TSDB samples, surviving through WAL and block storage,
// then being selected by PromQL and returned as an HTTP API result.

import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "exporter", class: "box", x: 20, y: 120, label: "instrumented target\nGET /metrics response", width: 200, height: 64 },
    { id: "discovery", class: "box", x: 20, y: 300, label: "discovery.Manager\nTargetGroup updates", width: 200, height: 64 },
    { id: "scrape-manager", class: "box", x: 280, y: 280, label: "scrape.Manager\nScrapePool per scrape_config", width: 230, height: 68 },
    { id: "target-relabel", class: "box", x: 570, y: 280, label: "ScrapePool target relabeling\njob, instance + configured labels", width: 250, height: 68 },
    { id: "scrape-loop", class: "box", x: 570, y: 100, label: "scrape loop\nperiodic HTTP GET", width: 220, height: 64 },
    { id: "exposition", class: "box", x: 870, y: 100, label: "decode exposition response\nmetric names, labels, values, timestamps", width: 260, height: 68 },
    { id: "metric-relabel", class: "box", x: 870, y: 280, label: "metric relabeling\nper-sample keep / drop / rewrite", width: 250, height: 68 },
    { id: "appender", class: "box", x: 1170, y: 250, label: "TSDB Appender\nvalidate series + append samples; Commit", width: 250, height: 72 },
    { id: "head", class: "box", x: 1480, y: 170, label: "TSDB Head\nin-memory series + sample chunks", width: 230, height: 68 },
    { id: "wal", class: "box", x: 1480, y: 350, label: "WAL\nseries + sample records for replay", width: 230, height: 68 },
    { id: "blocks", class: "box", x: 1780, y: 350, label: "TSDB blocks\n2h chunks + index + meta.json", width: 240, height: 68 },
    { id: "query-api", class: "box", x: 1780, y: 80, label: "HTTP API\n/api/v1/query?query=...", width: 240, height: 64 },
    { id: "promql", class: "box", x: 1480, y: 20, label: "PromQL engine\nparse expression; execute instant query", width: 250, height: 68 },
    { id: "querier", class: "box", x: 1170, y: 20, label: "TSDB Querier\nselect series; lazy sample iterators", width: 250, height: 68 },
    { id: "response", class: "box", x: 2070, y: 80, label: "HTTP JSON response\nstatus + data.resultType + data.result", width: 260, height: 68 },
  ] satisfies Node[],
  edges: [
    { from: "discovery", to: "scrape-manager", label: "target group updates" },
    { from: "scrape-manager", to: "target-relabel", label: "targets for scrape_config" },
    { from: "target-relabel", to: "scrape-loop", label: "final target URL + labels" },
    { from: "scrape-loop", to: "exporter", label: "HTTP GET /metrics" },
    { from: "exporter", to: "exposition", label: "HTTP body + content type" },
    { from: "exposition", to: "metric-relabel", label: "parsed samples" },
    { from: "metric-relabel", to: "appender", label: "accepted labeled samples" },
    { from: "scrape-loop", to: "appender", label: "scrape report: up, duration" },
    { from: "appender", to: "head", label: "committed series + samples" },
    { from: "appender", to: "wal", label: "commit records for crash recovery" },
    { from: "head", to: "blocks", label: "head cut + compaction" },
    { from: "wal", to: "head", label: "replay on restart" },
    { from: "query-api", to: "promql", label: "query string + evaluation time" },
    { from: "promql", to: "querier", label: "storage selectors / time bounds" },
    { from: "querier", to: "head", label: "recent in-memory samples" },
    { from: "querier", to: "blocks", label: "older indexed samples" },
    { from: "querier", to: "promql", label: "series iterators" },
    { from: "promql", to: "query-api", label: "vector, matrix, scalar or string" },
    { from: "query-api", to: "response", label: "JSON-encoded result + warnings" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
