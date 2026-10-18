import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

// OpenSearch seven-node deployment, based on:
// https://docs.opensearch.org/latest/tuning-your-cluster/
// https://docs.opensearch.org/latest/tuning-your-cluster/discovery-cluster-formation/
// https://docs.opensearch.org/latest/getting-started/intro/
// https://docs.opensearch.org/latest/ingest-pipelines/
// Shows the documented three cluster-manager-eligible nodes, three data nodes,
// and one coordinating-only node, with client traffic and cluster coordination.

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const nodes = [
  { id: "zone-a", class: "box", x: 30, y: 70, label: "Availability zone A", width: 350, height: 400, fill: "#eff6ff" },
  { id: "zone-b", class: "box", x: 410, y: 70, label: "Availability zone B", width: 350, height: 400, fill: "#f0fdf4" },
  { id: "zone-c", class: "box", x: 790, y: 70, label: "Availability zone C", width: 350, height: 400, fill: "#fff7ed" },
  { id: "client", class: "box", x: 30, y: 10, label: "Applications / API clients", width: 190 },
  { id: "dashboards", class: "box", x: 270, y: 10, label: "OpenSearch Dashboards", width: 190 },
  { id: "prepper", class: "box", x: 520, y: 10, label: "OpenSearch Data Prepper", width: 210 },
  { id: "external", class: "box", x: 790, y: 10, label: "External sources", width: 170 },
  { id: "manager-a-host", class: "box", x: 55, y: 130, parent: "zone-a", label: "OpenSearch node\ncluster_manager eligible", width: 260, height: 64 },
  { id: "manager-b-host", class: "box", x: 455, y: 130, parent: "zone-b", label: "OpenSearch node\ncluster_manager eligible", width: 260, height: 64 },
  { id: "manager-c-host", class: "box", x: 835, y: 130, parent: "zone-c", label: "OpenSearch node\ncluster_manager eligible", width: 260, height: 64 },
  { id: "manager-election", class: "box", x: 450, y: 220, label: "Voting quorum elects\none active cluster manager", width: 260, height: 64, fill: "#fef3c7" },
  { id: "cluster-state", class: "box", x: 450, y: 320, label: "Authoritative cluster state\nnode membership · index metadata\nshard allocation", width: 260, height: 82 },
  { id: "data-a-host", class: "box", x: 55, y: 275, parent: "zone-a", label: "opensearch-d1\nroles: data, ingest", width: 260, height: 62 },
  { id: "data-b-host", class: "box", x: 455, y: 275, parent: "zone-b", label: "opensearch-d2\nroles: data, ingest", width: 260, height: 62 },
  { id: "data-c-host", class: "box", x: 835, y: 275, parent: "zone-c", label: "OpenSearch data node\nroles: data, ingest", width: 260, height: 62 },
  { id: "shards-a", class: "box", x: 55, y: 370, parent: "zone-a", label: "Local index shards\nprimary / replica copies", width: 260, height: 62, fill: "#dbeafe" },
  { id: "shards-b", class: "box", x: 455, y: 370, parent: "zone-b", label: "Local index shards\nprimary / replica copies", width: 260, height: 62, fill: "#dcfce7" },
  { id: "shards-c", class: "box", x: 835, y: 370, parent: "zone-c", label: "Local index shards\nprimary / replica copies", width: 260, height: 62, fill: "#ffedd5" },
  { id: "coordinator-host", class: "box", x: 1180, y: 125, parent: "cluster", label: "opensearch-c1\nnode.roles: []\ncoordinating only", width: 260, height: 72 },
  { id: "cluster", class: "box", x: 1160, y: 75, label: "OpenSearch cluster", width: 300, height: 360, fill: "#f8fafc" },
  { id: "http", class: "box", x: 1210, y: 250, parent: "cluster", label: "HTTP request interface\nclient REST API", width: 210, height: 58 },
  { id: "transport", class: "box", x: 1210, y: 345, parent: "cluster", label: "Transport layer\ndiscovery · state · shard requests", width: 210, height: 66 },
  { id: "seed-hosts", class: "box", x: 1480, y: 100, label: "discovery.seed_hosts\nmanager-eligible addresses", width: 230, height: 62 },
  { id: "initial-voters", class: "box", x: 1480, y: 205, label: "cluster.initial_cluster_manager_nodes\nbootstrap voter names", width: 250, height: 64 },
  { id: "config", class: "box", x: 1480, y: 315, label: "opensearch.yml on each node\ncluster.name · node.roles\nnetwork.bind_host", width: 250, height: 76 },
  { id: "ingest-pipeline", class: "box", x: 790, y: 520, label: "Optional ingest pipeline\nprocessors run on ingest-role nodes", width: 270, height: 70, fill: "#fef3c7" },
  { id: "query-fanout", class: "box", x: 1160, y: 520, label: "Shard-level search / indexing\nfan-out and response aggregation", width: 300, height: 70, fill: "#e0e7ff" },
  { id: "return", class: "box", x: 30, y: 520, label: "Search response / write result", width: 230 },
] satisfies Node[];

const sketch = {
  nodes,
  edges: [
    { from: "client", to: "http", label: "REST search / index request" },
    { from: "dashboards", to: "http", label: "API calls; target ingest, coordinator, or data node" },
    { from: "external", to: "prepper", label: "source events / documents" },
    { from: "prepper", to: "http", label: "processed documents to OpenSearch" },
    { from: "http", to: "coordinator-host", label: "HTTP entry; coordinating node routes request" },
    { from: "coordinator-host", to: "query-fanout", label: "dispatch to relevant shards" },
    { from: "query-fanout", to: "data-a-host", label: "shard operation" },
    { from: "query-fanout", to: "data-b-host", label: "shard operation" },
    { from: "query-fanout", to: "data-c-host", label: "shard operation" },
    { from: "data-a-host", to: "shards-a", label: "read / write local shards" },
    { from: "data-b-host", to: "shards-b", label: "read / write local shards" },
    { from: "data-c-host", to: "shards-c", label: "read / write local shards" },
    { from: "shards-a", to: "shards-b", label: "replication / shard recovery" },
    { from: "shards-b", to: "shards-c", label: "replication / shard recovery" },
    { from: "data-a-host", to: "ingest-pipeline", label: "index-time processing (if pipeline used)" },
    { from: "ingest-pipeline", to: "shards-a", label: "processed document" },
    { from: "ingest-pipeline", to: "shards-b", label: "processed document" },
    { from: "ingest-pipeline", to: "shards-c", label: "processed document" },
    { from: "query-fanout", to: "coordinator-host", label: "collect shard results; aggregate" },
    { from: "coordinator-host", to: "return", label: "HTTP response" },
    { from: "manager-a-host", to: "manager-election", label: "participates in voting" },
    { from: "manager-b-host", to: "manager-election", label: "participates in voting" },
    { from: "manager-c-host", to: "manager-election", label: "participates in voting" },
    { from: "manager-election", to: "cluster-state", label: "one elected leader publishes state" },
    { from: "cluster-state", to: "manager-a-host", label: "state updates; health checks" },
    { from: "cluster-state", to: "manager-b-host", label: "state updates; health checks" },
    { from: "cluster-state", to: "manager-c-host", label: "state updates; health checks" },
    { from: "cluster-state", to: "data-a-host", label: "shard allocation / cluster state" },
    { from: "cluster-state", to: "data-b-host", label: "shard allocation / cluster state" },
    { from: "cluster-state", to: "data-c-host", label: "shard allocation / cluster state" },
    { from: "seed-hosts", to: "manager-a-host", label: "unicast discovery seed" },
    { from: "seed-hosts", to: "manager-b-host", label: "unicast discovery seed" },
    { from: "seed-hosts", to: "manager-c-host", label: "unicast discovery seed" },
    { from: "initial-voters", to: "manager-election", label: "initial bootstrap names" },
    { from: "config", to: "manager-a-host", label: "local node settings" },
    { from: "config", to: "data-a-host", label: "local node settings" },
    { from: "config", to: "coordinator-host", label: "empty roles list" },
    { from: "manager-a-host", to: "transport", label: "inter-node transport" },
    { from: "data-a-host", to: "transport", label: "inter-node transport" },
    { from: "coordinator-host", to: "transport", label: "inter-node transport" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
