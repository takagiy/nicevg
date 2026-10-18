import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// Sources: https://airflow.apache.org/docs/helm-chart/stable/index.html
// https://airflow.apache.org/docs/apache-airflow/stable/concepts/overview.html
// https://airflow.apache.org/docs/apache-airflow/stable/installation/upgrading_to_airflow3.html
// This sketch shows an Apache Airflow 3 deployment made with the official Helm chart,
// configured with CeleryExecutor: Kubernetes workloads, shared services, and task/API paths.
const nodes: Node[] = [
  { id: "operator", class: "box", x: 20, y: 125, label: "Deployment operator" },
  { id: "helm", class: "box", x: 190, y: 125, label: "Helm 3\napache-airflow/airflow" },
  { id: "cluster", class: "box", x: 390, y: 25, label: "Kubernetes cluster", width: 1130, height: 800, fill: "#eff6ff" },
  { id: "namespace", class: "box", x: 420, y: 70, label: "airflow namespace", width: 1060, height: 720, fill: "#f8fafc" },
  { id: "ingress", class: "box", x: 450, y: 120, label: "Ingress / load balancer", parent: "namespace", fill: "#dbeafe" },
  { id: "api-service", class: "box", x: 660, y: 120, label: "api-server Service :8080", parent: "namespace" },
  { id: "api-server", class: "box", x: 660, y: 195, label: "API server Pod\nREST API + UI + Execution API", parent: "namespace", fill: "#dcfce7" },
  { id: "scheduler", class: "box", x: 920, y: 120, label: "Scheduler Pod\nCeleryExecutor", parent: "namespace", fill: "#dcfce7" },
  { id: "dag-processor", class: "box", x: 920, y: 260, label: "DAG processor Pod", parent: "namespace", fill: "#dcfce7" },
  { id: "triggerer", class: "box", x: 920, y: 385, label: "Triggerer Pod\ndeferred tasks", parent: "namespace", fill: "#dcfce7" },
  { id: "celery-deployment", class: "box", x: 1180, y: 90, label: "Celery worker Deployment", width: 255, height: 300, parent: "namespace", fill: "#ecfccb" },
  { id: "worker-a", class: "box", x: 1200, y: 165, label: "Celery worker Pod A", parent: "celery-deployment" },
  { id: "worker-b", class: "box", x: 1200, y: 255, label: "Celery worker Pod B", parent: "celery-deployment" },
  { id: "worker-task", class: "box", x: 1180, y: 425, label: "Task subprocess\nper task instance", parent: "namespace", fill: "#fef3c7" },
  { id: "redis-service", class: "box", x: 660, y: 390, label: "redis Service :6379", parent: "namespace" },
  { id: "redis", class: "box", x: 660, y: 455, label: "Redis Pod\nCelery broker", parent: "namespace", fill: "#fee2e2" },
  { id: "pgbouncer-service", class: "box", x: 660, y: 555, label: "pgbouncer Service :6543", parent: "namespace" },
  { id: "pgbouncer", class: "box", x: 660, y: 615, label: "PgBouncer Pod\nconnection pool", parent: "namespace", fill: "#f3e8ff" },
  { id: "postgres", class: "box", x: 920, y: 555, label: "PostgreSQL Pod\nAirflow metadata DB", parent: "namespace", fill: "#f3e8ff" },
  { id: "metadata", class: "box", x: 1180, y: 555, label: "Metadata records\nDAGs · runs · tasks · variables", parent: "namespace", fill: "#fef9c3" },
  { id: "migrate-job", class: "box", x: 450, y: 650, label: "migrate-database Job", parent: "namespace" },
  { id: "create-user-job", class: "box", x: 450, y: 715, label: "create-user Job", parent: "namespace" },
  { id: "dag-repo", class: "box", x: 20, y: 335, label: "DAG bundle source\n(local folder / Git bundle)" },
  { id: "browser", class: "box", x: 20, y: 225, label: "Operations user / browser" },
  { id: "external-system", class: "box", x: 20, y: 495, label: "Task external systems\nprovider APIs / data stores" },
  { id: "flower", class: "box", x: 450, y: 300, label: "Flower Pod + Service\nCelery monitoring :5555", parent: "namespace" },
  { id: "statsd", class: "box", x: 450, y: 430, label: "StatsD exporter Pod\nAirflow metrics :9102", parent: "namespace" },
  { id: "prometheus", class: "box", x: 20, y: 635, label: "Prometheus" },
  { id: "secrets", class: "box", x: 920, y: 690, label: "Kubernetes Secrets\nDB / broker URLs · Fernet · JWT", parent: "namespace" },
];

const edges = [
  { from: "operator", to: "helm", label: "helm upgrade --install" },
  { from: "helm", to: "cluster", label: "creates chart resources" },
  { from: "browser", to: "ingress", label: "HTTPS" },
  { from: "ingress", to: "api-service", label: "HTTP :8080" },
  { from: "api-service", to: "api-server", label: "routes requests" },
  { from: "api-server", to: "browser", label: "UI + REST responses" },
  { from: "dag-repo", to: "dag-processor", label: "DAG bundle files" },
  { from: "dag-repo", to: "worker-a", label: "task's DAG bundle version" },
  { from: "dag-repo", to: "worker-b", label: "task's DAG bundle version" },
  { from: "dag-processor", to: "pgbouncer-service", label: "serialized DAGs" },
  { from: "scheduler", to: "pgbouncer-service", label: "read state; write scheduling state" },
  { from: "triggerer", to: "pgbouncer-service", label: "trigger state" },
  { from: "api-server", to: "pgbouncer-service", label: "API/UI metadata access" },
  { from: "worker-a", to: "api-server", label: "Execution API: state, logs, XCom, resources" },
  { from: "worker-b", to: "api-server", label: "Execution API: state, logs, XCom, resources" },
  { from: "pgbouncer-service", to: "pgbouncer", label: "database connections" },
  { from: "pgbouncer", to: "postgres", label: "pooled PostgreSQL sessions" },
  { from: "postgres", to: "metadata", label: "persist / query" },
  { from: "scheduler", to: "redis-service", label: "enqueue task messages" },
  { from: "redis-service", to: "redis", label: "Celery broker connection" },
  { from: "redis", to: "worker-a", label: "deliver task messages" },
  { from: "redis", to: "worker-b", label: "deliver task messages" },
  { from: "worker-a", to: "worker-task", label: "starts task subprocess" },
  { from: "worker-b", to: "worker-task", label: "starts task subprocess" },
  { from: "worker-task", to: "external-system", label: "operator/provider calls" },
  { from: "worker-a", to: "redis", label: "task result / acknowledgement" },
  { from: "worker-b", to: "redis", label: "task result / acknowledgement" },
  { from: "flower", to: "redis-service", label: "Celery broker inspection" },
  { from: "migrate-job", to: "pgbouncer-service", label: "Airflow DB migrations" },
  { from: "create-user-job", to: "pgbouncer-service", label: "administrator account" },
  { from: "scheduler", to: "secrets", label: "DB / broker credentials" },
  { from: "api-server", to: "secrets", label: "JWT / Fernet keys" },
  { from: "statsd", to: "prometheus", label: "scrape :9102" },
  { from: "scheduler", to: "statsd", label: "StatsD metrics" },
  { from: "dag-processor", to: "statsd", label: "StatsD metrics" },
  { from: "triggerer", to: "statsd", label: "StatsD metrics" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
