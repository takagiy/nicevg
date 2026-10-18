import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

// Apache Airflow deployment topology, comparing the documented CeleryExecutor
// Docker Compose services in 2.10 and 3.0. The key change is that Airflow 3
// workers use the Task Execution API instead of connecting to the metadata DB.
// Sources:
// https://airflow.apache.org/docs/apache-airflow/2.10.0/howto/docker-compose/index.html
// https://airflow.apache.org/docs/apache-airflow/3.0.0/howto/docker-compose/index.html
// https://airflow.apache.org/docs/apache-airflow/3.0.0/installation/upgrading_to_airflow3.html

type Boundary = SketchNode & { class: "boundary"; title: string; width: number; height: number; fill: string };
type Service = SketchNode & { class: "service" | "store" | "client"; label: string; width: number; height: number };
type AirflowNode = Boundary | Service;

const boundary = ({ title, width, height, fill }: Boundary) => svg`
  <rect data-bounds="true" width="${width}" height="${height}" rx="12" fill="${fill}" stroke="#64748b" stroke-width="1.5"/>
  ${text(title, 17).at({ left: 18, top: 12 })}`;

const service = ({ label, width, height, class: kind }: Service) => {
  const lines = text(label, 13);
  const fill = kind === "store" ? "#fff7ed" : kind === "client" ? "#fefce8" : "#eff6ff";
  const shape = kind === "store"
    ? svg`<path d="M 0 12 C 0 0 ${width} 0 ${width} 12 L ${width} ${height - 12} C ${width} ${height} 0 ${height} 0 ${height - 12} Z" fill="${fill}" stroke="#334155"/><ellipse cx="${width / 2}" cy="12" rx="${width / 2}" ry="12" fill="${fill}" stroke="#334155"/>`
    : kind === "client"
      ? svg`<ellipse data-bounds="true" cx="${width / 2}" cy="${height / 2}" rx="${width / 2}" ry="${height / 2}" fill="${fill}" stroke="#334155"/>`
      : svg`<rect data-bounds="true" width="${width}" height="${height}" rx="7" fill="${fill}" stroke="#334155"/>`;
  return svg`${shape}${lines.at({ cx: width / 2, cy: height / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "airflow-210", class: "boundary", x: 20, y: 120, width: 650, height: 600, title: "Airflow 2.10 · Docker Compose", fill: "#f8fafc" },
    { id: "airflow-300", class: "boundary", x: 720, y: 120, width: 650, height: 600, title: "Airflow 3.0 · Docker Compose", fill: "#f8fafc" },
    { id: "operator", class: "client", x: 640, y: 22, width: 150, height: 56, label: "Browser / REST client" },

    { id: "v2-web", class: "service", parent: "airflow-210", x: 45, y: 185, width: 270, height: 78, label: "airflow-webserver\nUI + REST API :8080" },
    { id: "v2-scheduler", class: "service", parent: "airflow-210", x: 365, y: 185, width: 270, height: 78, label: "airflow-scheduler\nDAGs → task scheduling" },
    { id: "v2-worker", class: "service", parent: "airflow-210", x: 45, y: 330, width: 270, height: 78, label: "airflow-worker\nCelery task execution" },
    { id: "v2-triggerer", class: "service", parent: "airflow-210", x: 365, y: 330, width: 270, height: 78, label: "airflow-triggerer\ndeferrable task event loop" },
    { id: "v2-init", class: "service", parent: "airflow-210", x: 45, y: 485, width: 270, height: 78, label: "airflow-init\nDB migration + first user" },
    { id: "v2-db", class: "store", parent: "airflow-210", x: 365, y: 485, width: 270, height: 78, label: "postgres\nmetadata database" },
    { id: "v2-redis", class: "store", parent: "airflow-210", x: 205, y: 610, width: 270, height: 78, label: "redis\nCelery task broker" },

    { id: "v3-api", class: "service", parent: "airflow-300", x: 745, y: 185, width: 270, height: 78, label: "airflow-api-server\nUI + REST + Task Execution API" },
    { id: "v3-scheduler", class: "service", parent: "airflow-300", x: 1065, y: 185, width: 270, height: 78, label: "airflow-scheduler\nDAGs → task scheduling" },
    { id: "v3-worker", class: "service", parent: "airflow-300", x: 745, y: 330, width: 270, height: 78, label: "airflow-worker\nTask SDK subprocesses" },
    { id: "v3-dagproc", class: "service", parent: "airflow-300", x: 1065, y: 330, width: 270, height: 78, label: "airflow-dag-processor\nstandalone DAG parsing" },
    { id: "v3-triggerer", class: "service", parent: "airflow-300", x: 745, y: 485, width: 270, height: 78, label: "airflow-triggerer\ndeferrable task event loop" },
    { id: "v3-init", class: "service", parent: "airflow-300", x: 1065, y: 485, width: 270, height: 78, label: "airflow-init\nDB migration + first user" },
    { id: "v3-db", class: "store", parent: "airflow-300", x: 745, y: 610, width: 270, height: 78, label: "postgres\nmetadata database" },
    { id: "v3-redis", class: "store", parent: "airflow-300", x: 1065, y: 610, width: 270, height: 78, label: "redis\nCelery task broker" },
  ],
  edges: [
    { from: "operator", to: "v2-web" },
    { from: "v2-web", to: "v2-db" }, { from: "v2-scheduler", to: "v2-db" },
    { from: "v2-worker", to: "v2-db" }, { from: "v2-triggerer", to: "v2-db" }, { from: "v2-init", to: "v2-db" },
    { from: "v2-scheduler", to: "v2-redis" }, { from: "v2-redis", to: "v2-worker" },

    { from: "operator", to: "v3-api" },
    { from: "v3-api", to: "v3-db" }, { from: "v3-scheduler", to: "v3-db" },
    { from: "v3-dagproc", to: "v3-db" }, { from: "v3-init", to: "v3-db" },
    { from: "v3-scheduler", to: "v3-redis" }, { from: "v3-redis", to: "v3-worker" },
    { from: "v3-worker", to: "v3-api", label: "task state, heartbeat, XCom, resources" },
  ],
  style: { boundary, service, store: service, client: service },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<AirflowNode>;

export default sketch;
