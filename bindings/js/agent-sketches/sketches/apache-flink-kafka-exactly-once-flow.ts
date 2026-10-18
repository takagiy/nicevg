// Apache Flink; sources: https://flink.apache.org/2018/02/28/an-overview-of-end-to-end-exactly-once-processing-in-apache-flink-with-apache-kafka-too/
// and https://nightlies.apache.org/flink/flink-docs-stable/docs/ops/state/checkpoints/.
// Shows Kafka input records through a windowed Flink computation to transactional Kafka output,
// including checkpoint snapshots, sink pre-commit, successful commit, and failure recovery.
import type { Sketch, SketchNode, Style } from "nicevg";
import { box } from "nicevg/style/simple";

export default {
  nodes: [
    { id: "input-topic", class: "box", x: 20, y: 170, label: "Kafka input topic\npartition records" },
    { id: "source", class: "box", x: 220, y: 170, label: "KafkaConsumer\nsource offsets" },
    { id: "window", class: "box", x: 430, y: 170, label: "Windowed\naggregation" },
    { id: "window-state", class: "box", x: 430, y: 310, label: "State backend\nwindow sums" },
    { id: "sink", class: "box", x: 660, y: 170, label: "KafkaProducer\ntransactional sink" },
    { id: "output-topic", class: "box", x: 900, y: 170, label: "Kafka output topic\ncommitted results" },
    { id: "jobmanager", class: "box", x: 230, y: 20, label: "JobManager\ncheckpoint trigger" },
    { id: "barrier", class: "box", x: 460, y: 20, label: "Checkpoint barrier\nstream cut" },
    { id: "checkpoint", class: "box", x: 690, y: 20, label: "Completed checkpoint\nstate + input position" },
    { id: "storage", class: "box", x: 900, y: 20, label: "Persistent checkpoint\nstorage (S3 or HDFS)" },
    { id: "abort", class: "box", x: 660, y: 330, label: "Abort sink transaction\non pre-commit failure" },
    { id: "restore", class: "box", x: 230, y: 430, label: "Restart from latest\nsuccessful checkpoint" },
  ],
  edges: [
    { from: "input-topic", to: "source", label: "Kafka records" },
    { from: "source", to: "window", label: "deserialized events" },
    { from: "window", to: "window-state", label: "update keyed window sums" },
    { from: "window-state", to: "window", label: "aggregation state" },
    { from: "window", to: "sink", label: "window results" },
    { from: "sink", to: "output-topic", label: "transaction writes; visible on commit" },
    { from: "jobmanager", to: "source", label: "trigger checkpoint" },
    { from: "jobmanager", to: "barrier", label: "inject barrier" },
    { from: "barrier", to: "source", label: "snapshot input offsets" },
    { from: "barrier", to: "window", label: "snapshot operator state" },
    { from: "barrier", to: "sink", label: "pre-commit external transaction" },
    { from: "window-state", to: "checkpoint", label: "state snapshot" },
    { from: "sink", to: "checkpoint", label: "pre-committed transaction state" },
    { from: "checkpoint", to: "storage", label: "asynchronous durable snapshot" },
    { from: "checkpoint", to: "sink", label: "checkpoint-completed callback: commit" },
    { from: "barrier", to: "abort", label: "any pre-commit fails" },
    { from: "abort", to: "restore", label: "rollback to prior completed checkpoint" },
    { from: "storage", to: "restore", label: "restore offsets and operator state" },
    { from: "restore", to: "source", label: "resume input position" },
  ],
  style: { box: box as Style<SketchNode> },
} satisfies Sketch;
