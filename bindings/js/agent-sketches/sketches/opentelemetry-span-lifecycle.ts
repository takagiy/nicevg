import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// OpenTelemetry — https://opentelemetry.io/docs/specs/otel/trace/api/ and
// https://opentelemetry.io/docs/specs/otel/trace/sdk/
// Follows one Span from Tracer creation through sampling, recording and End(),
// then through processor/exporter handling, including drop and export failure paths.

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
};

const nodes: Node[] = [
  { id: "instrumentation", class: "box", x: 20, y: 220, label: "Instrumentation / application\nTracer.startSpan(name, parent Context,\nkind, initial attributes, links, start time)", fill: "#f1f5f9", width: 240 },
  { id: "provider", class: "box", x: 300, y: 20, label: "TracerProvider\nregistered SpanProcessors\nSampler configuration", fill: "#e0e7ff", width: 210 },
  { id: "tracer", class: "box", x: 300, y: 200, label: "Tracer\nonly API for creating a Span", fill: "#dbeafe", width: 210 },
  { id: "parent_context", class: "box", x: 20, y: 20, label: "Parent Context\nvalid parent SpanContext or root", fill: "#f1f5f9", width: 220 },
  { id: "trace_id", class: "box", x: 570, y: 20, label: "TraceId\ninherited from valid parent\nor generated for root", fill: "#f8fafc", width: 210 },
  { id: "sampler", class: "box", x: 570, y: 180, label: "Sampler.ShouldSample\nparent Context, TraceId, name, kind,\ninitial attributes and links", fill: "#fef3c7", width: 240 },
  { id: "sampling_result", class: "box", x: 860, y: 180, label: "SamplingResult\nDecision + attributes + Tracestate", fill: "#fef3c7", width: 210 },
  { id: "drop", class: "box", x: 1140, y: 20, label: "DROP\nIsRecording=false\nSampled=false", fill: "#fee2e2", width: 200 },
  { id: "record_only", class: "box", x: 1140, y: 150, label: "RECORD_ONLY\nIsRecording=true\nSampled=false", fill: "#ffedd5", width: 200 },
  { id: "record_sample", class: "box", x: 1140, y: 280, label: "RECORD_AND_SAMPLE\nIsRecording=true\nSampled=true", fill: "#dcfce7", width: 220 },
  { id: "span_id", class: "box", x: 860, y: 350, label: "New SpanId\ncreated independently of decision", fill: "#f8fafc", width: 220 },
  { id: "nonrecording", class: "box", x: 1430, y: 20, label: "NonRecordingSpan\ncontext available; data APIs no-op\nnot sent to SpanProcessors", fill: "#fee2e2", width: 230 },
  { id: "recording_span", class: "box", x: 1430, y: 220, label: "Recording Span\nSpanContext: TraceId, SpanId,\nTraceFlags, TraceState\nstart timestamp; parent SpanContext", fill: "#dcfce7", width: 250 },
  { id: "attributes", class: "box", x: 1740, y: 20, label: "SetAttribute(s)\nkey/value attributes\nlimits may drop excess", fill: "#f8fafc", width: 210 },
  { id: "events", class: "box", x: 1740, y: 140, label: "AddEvent / RecordException\nname, timestamp, attributes", fill: "#f8fafc", width: 210 },
  { id: "status", class: "box", x: 1740, y: 260, label: "SetStatus\nUnset / Ok / Error\noptional description", fill: "#f8fafc", width: 210 },
  { id: "children", class: "box", x: 1740, y: 380, label: "Child Span\nsame TraceId; parent SpanContext\ncreated independently", fill: "#e0f2fe", width: 220 },
  { id: "end_call", class: "box", x: 1430, y: 480, label: "Span.End(optional timestamp)\nsecond End has no effect", fill: "#dbeafe", width: 250 },
  { id: "on_ending", class: "box", x: 1740, y: 500, label: "SpanProcessor.OnEnding\nend time computed; Span still mutable\nduring callbacks", fill: "#ede9fe", width: 240 },
  { id: "ended", class: "box", x: 1430, y: 650, label: "Ended Span\nend timestamp set\nnon-recording; further writes ignored", fill: "#e2e8f0", width: 250 },
  { id: "readable", class: "box", x: 1740, y: 680, label: "ReadableSpan / SpanData\nended flag + timestamps, context,\nattributes, events, status, resource, scope", fill: "#ede9fe", width: 260 },
  { id: "on_start", class: "box", x: 860, y: 500, label: "SpanProcessor.OnStart\nsynchronous; registered order\nrecording spans only", fill: "#ede9fe", width: 240 },
  { id: "processor_choice", class: "box", x: 860, y: 680, label: "Configured SpanProcessor pipeline\nprocessors run in registration order", fill: "#ede9fe", width: 250 },
  { id: "simple", class: "box", x: 1140, y: 560, label: "SimpleSpanProcessor\nfinished span → exporter immediately\nserializes Export calls", fill: "#f3e8ff", width: 230 },
  { id: "batch", class: "box", x: 1140, y: 740, label: "BatchSpanProcessor\nconvert ended span to export form\nappend to queue", fill: "#f3e8ff", width: 230 },
  { id: "queue", class: "box", x: 1430, y: 840, label: "Batch queue\nfinished spans accumulate\nsize / schedule triggers", fill: "#faf5ff", width: 220 },
  { id: "batch_export", class: "box", x: 1740, y: 840, label: "Export(batch)\nreadable span data", fill: "#fee2e2", width: 210 },
  { id: "exporter", class: "box", x: 2020, y: 840, label: "SpanExporter\nprotocol encode + transmit", fill: "#fee2e2", width: 210 },
  { id: "success", class: "box", x: 2300, y: 760, label: "Success\ndelivered to destination", fill: "#dcfce7", width: 200 },
  { id: "failure", class: "box", x: 2300, y: 900, label: "Failure\nbatch dropped by processor\nretry logic is exporter/protocol specific", fill: "#fee2e2", width: 240 },
  { id: "backend", class: "box", x: 2580, y: 760, label: "Trace backend\nreceives sampled spans", fill: "#f1f5f9", width: 210 },
  { id: "flush_shutdown", class: "box", x: 2020, y: 1040, label: "ForceFlush / Shutdown\nprocessor drains or completes pending work\nwithin timeout", fill: "#e0e7ff", width: 260 },
];

const sketch = {
  nodes,
  edges: [
    { from: "instrumentation", to: "tracer", label: "startSpan arguments" },
    { from: "provider", to: "tracer", label: "Tracer from provider" },
    { from: "provider", to: "sampler", label: "configured Sampler" },
    { from: "parent_context", to: "trace_id", label: "parent TraceId if valid" },
    { from: "tracer", to: "trace_id", label: "generate root TraceId before sampling" },
    { from: "trace_id", to: "sampler", label: "TraceId input" },
    { from: "tracer", to: "sampler", label: "name / kind / initial attrs / links" },
    { from: "sampler", to: "sampling_result", label: "Decision + initial data" },
    { from: "sampling_result", to: "drop", label: "DROP" },
    { from: "sampling_result", to: "record_only", label: "RECORD_ONLY" },
    { from: "sampling_result", to: "record_sample", label: "RECORD_AND_SAMPLE" },
    { from: "tracer", to: "span_id", label: "generate regardless of decision" },
    { from: "drop", to: "nonrecording", label: "no recording" },
    { from: "record_only", to: "recording_span", label: "record; sampled flag clear" },
    { from: "record_sample", to: "recording_span", label: "record; sampled flag set" },
    { from: "span_id", to: "recording_span", label: "SpanContext identity" },
    { from: "span_id", to: "nonrecording", label: "unique ID also on dropped Span" },
    { from: "recording_span", to: "attributes", label: "SetAttribute" },
    { from: "recording_span", to: "events", label: "AddEvent / RecordException" },
    { from: "recording_span", to: "status", label: "SetStatus" },
    { from: "recording_span", to: "children", label: "parent Context; child shares TraceId" },
    { from: "recording_span", to: "end_call", label: "operation completes" },
    { from: "end_call", to: "on_ending", label: "record end timestamp; callbacks" },
    { from: "on_ending", to: "ended", label: "finish End()" },
    { from: "ended", to: "readable", label: "snapshot for processors" },
    { from: "recording_span", to: "on_start", label: "OnStart after creation" },
    { from: "provider", to: "processor_choice", label: "registered processors" },
    { from: "on_start", to: "processor_choice", label: "start callback" },
    { from: "readable", to: "processor_choice", label: "OnEnd receives ended span" },
    { from: "processor_choice", to: "simple", label: "simple pipeline" },
    { from: "processor_choice", to: "batch", label: "batch pipeline" },
    { from: "simple", to: "batch_export", label: "one ended span as batch" },
    { from: "batch", to: "queue", label: "enqueue export representation" },
    { from: "queue", to: "batch_export", label: "flush batch on size / schedule" },
    { from: "batch_export", to: "exporter", label: "Export(batch)" },
    { from: "exporter", to: "success", label: "ExportResult.Success" },
    { from: "exporter", to: "failure", label: "ExportResult.Failure" },
    { from: "success", to: "backend", label: "wire delivery" },
    { from: "failure", to: "queue", label: "no SDK retry guarantee" },
    { from: "flush_shutdown", to: "batch", label: "ForceFlush / Shutdown" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
