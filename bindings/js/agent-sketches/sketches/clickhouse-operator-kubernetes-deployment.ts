import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// Sources:
// https://github.com/ClickHouse/clickhouse-operator
// https://github.com/ClickHouse/clickhouse-operator/blob/main/docs/install/kubectl.mdx
// https://github.com/ClickHouse/clickhouse-docs/blob/main/docs/kubernetes-operator/03_guides/02_configuration.mdx
// https://raw.githubusercontent.com/ClickHouse/clickhouse-operator/main/examples/minimal.yaml
// This sketch explains the official ClickHouse Operator's Kubernetes deployment shape: its
// controller and APIs manage a two-replica ClickHouse cluster coordinated by a three-member Keeper cluster.

const nodes: Node[] = [
  { id: "kubectl", class: "box", x: 20, y: 80, label: "Cluster administrator\nkubectl apply" },
  { id: "application", class: "box", x: 20, y: 300, label: "Application / analyst\nClickHouse client" },
  { id: "prometheus", class: "box", x: 20, y: 550, label: "Prometheus" },
  { id: "cluster", class: "box", x: 250, y: 20, label: "Kubernetes cluster ≥ 1.28", width: 1230, height: 850, fill: "#eff6ff" },
  { id: "operator-ns", class: "box", x: 280, y: 70, label: "clickhouse-operator-system namespace", width: 400, height: 430, parent: "cluster", fill: "#f8fafc" },
  { id: "crds", class: "box", x: 310, y: 130, label: "CRDs\nClickHouseCluster · KeeperCluster", parent: "operator-ns" },
  { id: "service-account", class: "box", x: 310, y: 220, label: "ServiceAccount + RBAC", parent: "operator-ns" },
  { id: "operator", class: "box", x: 490, y: 220, label: "operator controller manager\nDeployment / Pod", parent: "operator-ns", fill: "#dcfce7" },
  { id: "webhook", class: "box", x: 490, y: 330, label: "Admission webhooks\nvalidate + default resources", parent: "operator-ns" },
  { id: "cert-manager", class: "box", x: 310, y: 410, label: "cert-manager\nwebhook TLS certificates", parent: "operator-ns" },
  { id: "operator-metrics", class: "box", x: 490, y: 420, label: "Operator metrics endpoint", parent: "operator-ns" },
  { id: "workload-ns", class: "box", x: 720, y: 70, label: "clickhouse namespace", width: 720, height: 740, parent: "cluster", fill: "#f0fdf4" },
  { id: "keeper-cr", class: "box", x: 750, y: 125, label: "KeeperCluster\nsample · replicas: 3", parent: "workload-ns", fill: "#fef3c7" },
  { id: "keeper-sts", class: "box", x: 750, y: 235, label: "Keeper StatefulSet", parent: "workload-ns" },
  { id: "keeper-a", class: "box", x: 750, y: 325, label: "Keeper Pod 0", parent: "workload-ns", fill: "#fee2e2" },
  { id: "keeper-b", class: "box", x: 915, y: 325, label: "Keeper Pod 1", parent: "workload-ns", fill: "#fee2e2" },
  { id: "keeper-c", class: "box", x: 1080, y: 325, label: "Keeper Pod 2", parent: "workload-ns", fill: "#fee2e2" },
  { id: "keeper-service", class: "box", x: 1260, y: 235, label: "Keeper peer/client Service\nTCP :9181", parent: "workload-ns" },
  { id: "keeper-volumes", class: "box", x: 750, y: 420, label: "3 data PVCs → PVs\nKeeper Raft state", parent: "workload-ns" },
  { id: "ch-cr", class: "box", x: 750, y: 520, label: "ClickHouseCluster\nsample · 1 shard × 2 replicas", parent: "workload-ns", fill: "#fef3c7" },
  { id: "ch-sts", class: "box", x: 750, y: 620, label: "ClickHouse StatefulSet", parent: "workload-ns" },
  { id: "ch-a", class: "box", x: 750, y: 705, label: "ClickHouse Pod 0-0", parent: "workload-ns", fill: "#dbeafe" },
  { id: "ch-b", class: "box", x: 945, y: 705, label: "ClickHouse Pod 0-1", parent: "workload-ns", fill: "#dbeafe" },
  { id: "ch-volumes", class: "box", x: 1130, y: 620, label: "2 data PVCs → PVs\nMergeTree data", parent: "workload-ns" },
  { id: "client-service", class: "box", x: 1260, y: 520, label: "ClickHouse client Service\nTCP :9000 · HTTP :8123", parent: "workload-ns" },
  { id: "keeper-peer", class: "box", x: 915, y: 455, label: "Keeper Raft peer quorum", parent: "workload-ns" },
  { id: "clickhouse-config", class: "box", x: 750, y: 455, label: "Generated server config\nKeeper endpoints + cluster topology", parent: "workload-ns" },
  { id: "storage-class", class: "box", x: 250, y: 740, label: "CSI StorageClass\nprovisions persistent volumes" },
  { id: "metrics-service", class: "box", x: 490, y: 510, label: "Metrics Service / scrape target", parent: "cluster" },
  { id: "client-ingress", class: "box", x: 20, y: 420, label: "Optional external LB / ingress\noperator does not require one" },
];

const edges = [
  { from: "kubectl", to: "crds", label: "install CRDs + controller manifests" },
  { from: "kubectl", to: "keeper-cr", label: "apply KeeperCluster YAML" },
  { from: "kubectl", to: "ch-cr", label: "apply ClickHouseCluster YAML" },
  { from: "operator", to: "service-account", label: "uses RBAC identity" },
  { from: "operator", to: "crds", label: "watches custom resources" },
  { from: "operator", to: "webhook", label: "registers validation/defaulting" },
  { from: "cert-manager", to: "webhook", label: "issues serving certificate" },
  { from: "operator", to: "keeper-sts", label: "reconcile KeeperCluster" },
  { from: "operator", to: "ch-sts", label: "reconcile ClickHouseCluster" },
  { from: "keeper-cr", to: "operator", label: "desired replicas + data PVC spec" },
  { from: "ch-cr", to: "operator", label: "replicas + keeperClusterRef" },
  { from: "keeper-sts", to: "keeper-a", label: "creates ordinal 0" },
  { from: "keeper-sts", to: "keeper-b", label: "creates ordinal 1" },
  { from: "keeper-sts", to: "keeper-c", label: "creates ordinal 2" },
  { from: "keeper-a", to: "keeper-peer", label: "Raft member" },
  { from: "keeper-b", to: "keeper-peer", label: "Raft member" },
  { from: "keeper-c", to: "keeper-peer", label: "Raft member" },
  { from: "keeper-peer", to: "keeper-service", label: "peer discovery + client access" },
  { from: "keeper-service", to: "ch-a", label: "Keeper protocol :9181" },
  { from: "keeper-service", to: "ch-b", label: "Keeper protocol :9181" },
  { from: "keeper-sts", to: "keeper-volumes", label: "mounts per-pod data PVC" },
  { from: "ch-sts", to: "ch-a", label: "creates shard 0 replica 0" },
  { from: "ch-sts", to: "ch-b", label: "creates shard 0 replica 1" },
  { from: "ch-sts", to: "ch-volumes", label: "mounts per-pod data PVC" },
  { from: "ch-cr", to: "clickhouse-config", label: "renders topology + Keeper reference" },
  { from: "clickhouse-config", to: "ch-a", label: "mounted configuration" },
  { from: "clickhouse-config", to: "ch-b", label: "mounted configuration" },
  { from: "ch-a", to: "client-service", label: "native / HTTP listener" },
  { from: "ch-b", to: "client-service", label: "native / HTTP listener" },
  { from: "application", to: "client-ingress", label: "optional external route" },
  { from: "client-ingress", to: "client-service", label: "TCP / HTTP" },
  { from: "application", to: "client-service", label: "in-cluster DNS + :9000 / :8123" },
  { from: "operator", to: "operator-metrics", label: "exposes controller metrics" },
  { from: "operator-metrics", to: "metrics-service", label: "Service endpoint" },
  { from: "metrics-service", to: "prometheus", label: "scrape when configured" },
  { from: "storage-class", to: "keeper-volumes", label: "dynamic PV provisioning" },
  { from: "storage-class", to: "ch-volumes", label: "dynamic PV provisioning" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
