import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// OpenTelemetry Collector — https://opentelemetry.io/docs/collector/architecture/,
// https://opentelemetry.io/docs/collector/components/, https://opentelemetry.io/docs/collector/components/extension/
// Shows how service pipeline configuration wires receivers, processors and exporters at startup,
// and how a health check extension serves the Collector alongside telemetry pipelines.

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
  parent?: string;
  width?: number;
  height?: number;
  stroke?: string;
};

const nodes: Node[] = [
  { id: "collector", class: "box", x: 260, y: 20, width: 1080, height: 520, label: "OpenTelemetry Collector process", fill: "#f8fafc", stroke: "#94a3b8" },
  { id: "config", class: "box", x: 20, y: 20, label: "Collector configuration\nreceivers / processors / exporters\nservice.pipelines + service.extensions", fill: "#e0e7ff" },
  { id: "service", class: "box", x: 290, y: 105, label: "service startup\nconstruct configured component instances", fill: "#dbeafe", parent: "collector" },
  { id: "app", class: "box", x: 20, y: 260, label: "Instrumented applications\nOTLP telemetry", fill: "#fae8ff" },
  { id: "otlp", class: "box", x: 290, y: 250, label: "OTLP receiver\ngRPC / HTTP", fill: "#dcfce7", parent: "collector" },
  { id: "zipkin", class: "box", x: 290, y: 390, label: "Zipkin receiver\ntrace ingestion", fill: "#dcfce7", parent: "collector" },
  { id: "memory", class: "box", x: 560, y: 250, label: "memory_limiter processor\nrefuse data under memory pressure", fill: "#fef3c7", parent: "collector" },
  { id: "batch", class: "box", x: 560, y: 390, label: "batch processor\nbuffer and group telemetry", fill: "#fef3c7", parent: "collector" },
  { id: "fanout", class: "box", x: 830, y: 315, label: "fanoutconsumer\ncopy each item to each exporter", fill: "#ffedd5", parent: "collector" },
  { id: "otlp_exporter", class: "box", x: 1090, y: 220, label: "OTLP exporter\nserialize and send telemetry", fill: "#fee2e2", parent: "collector" },
  { id: "zipkin_exporter", class: "box", x: 1090, y: 400, label: "Zipkin exporter\nconvert and send traces", fill: "#fee2e2", parent: "collector" },
  { id: "backend", class: "box", x: 1370, y: 220, label: "OTLP observability backend", fill: "#f1f5f9" },
  { id: "zipkin_backend", class: "box", x: 1370, y: 400, label: "Zipkin backend", fill: "#f1f5f9" },
  { id: "health", class: "box", x: 560, y: 70, label: "health_check extension\nHTTP health endpoint", fill: "#cffafe", parent: "collector" },
];

const sketch = {
  nodes,
  edges: [
    { from: "config", to: "service", label: "component IDs and pipeline order" },
    { from: "service", to: "otlp", label: "instantiate and connect" },
    { from: "service", to: "zipkin", label: "instantiate and connect" },
    { from: "service", to: "memory", label: "ordered processor instance" },
    { from: "service", to: "health", label: "start configured extension" },
    { from: "app", to: "otlp", label: "OTLP spans / metrics / logs" },
    { from: "otlp", to: "memory", label: "signal data" },
    { from: "zipkin", to: "memory", label: "trace data" },
    { from: "memory", to: "batch", label: "accepted telemetry" },
    { from: "batch", to: "fanout", label: "processed telemetry" },
    { from: "fanout", to: "otlp_exporter", label: "copy" },
    { from: "fanout", to: "zipkin_exporter", label: "copy" },
    { from: "otlp_exporter", to: "backend", label: "OTLP export request" },
    { from: "zipkin_exporter", to: "zipkin_backend", label: "Zipkin trace format" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
