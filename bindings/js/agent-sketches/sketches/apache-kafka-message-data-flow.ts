// Apache Kafka — https://kafka.apache.org/42/design/design/
// https://kafka.apache.org/protocol.html
// https://kafka.apache.org/42/implementation/distribution/
// This sketch follows keyed records from a producer through metadata lookup, partition leaders
// and replica logs, then through consumer-group fetch, application output and offset commits.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
  fill?: string;
  width?: number;
};

const sketch = {
  nodes: [
    { id: "source", class: "box", x: 20, y: 180, label: "Input application\nclick / event record", fill: "#eff6ff" },
    { id: "producer", class: "box", x: 220, y: 180, label: "KafkaProducer.send(record)\nkey + value + headers", fill: "#eff6ff" },
    { id: "serializer", class: "box", x: 220, y: 295, label: "key.serializer\nvalue.serializer" },
    { id: "partitioner", class: "box", x: 420, y: 180, label: "Partitioner\nkey hash → topic partition" },
    { id: "accumulator", class: "box", x: 630, y: 180, label: "RecordAccumulator\nper-partition batches\nbatch.size / linger.ms" },
    { id: "sender", class: "box", x: 840, y: 180, label: "Sender I/O thread\nProduceRequest per broker" },
    { id: "bootstrap", class: "box", x: 420, y: 20, label: "Bootstrap broker\nMetadataRequest" },
    { id: "metadata", class: "box", x: 630, y: 20, label: "Metadata cache\ntopic → partition leaders" },
    { id: "cluster", class: "box", x: 1080, y: 20, width: 730, label: "Kafka brokers — partitioned, replicated append logs", fill: "#f0fdf4" },
    { id: "broker-a", class: "box", x: 1110, y: 85, parent: "cluster", label: "Broker 1\nleader: clicks-0" },
    { id: "log-a", class: "box", x: 1110, y: 175, parent: "cluster", label: "clicks-0 log\nappend records at offsets" },
    { id: "broker-b", class: "box", x: 1370, y: 85, parent: "cluster", label: "Broker 2\nfollower: clicks-0" },
    { id: "replica-b", class: "box", x: 1370, y: 175, parent: "cluster", label: "clicks-0 replica log\nreplica fetch from leader" },
    { id: "broker-c", class: "box", x: 1630, y: 85, parent: "cluster", label: "Broker 3\nleader: clicks-1" },
    { id: "log-c", class: "box", x: 1630, y: 175, parent: "cluster", label: "clicks-1 log\nappend records at offsets" },
    { id: "ack", class: "box", x: 1110, y: 285, parent: "cluster", label: "ProduceResponse\nbase offset + error per partition" },
    { id: "group", class: "box", x: 1080, y: 400, width: 730, label: "Consumer group: click-enricher", fill: "#fff7ed" },
    { id: "coordinator", class: "box", x: 1110, y: 465, parent: "group", label: "Group coordinator\nFindCoordinator / JoinGroup" },
    { id: "assignment", class: "box", x: 1370, y: 465, parent: "group", label: "Partition assignment\nclicks-0 → Consumer A\nclicks-1 → Consumer B" },
    { id: "consumer-a", class: "box", x: 1110, y: 590, parent: "group", label: "Consumer A\nFetchRequest(offset 42)" },
    { id: "consumer-b", class: "box", x: 1370, y: 590, parent: "group", label: "Consumer B\nFetchRequest(offset 18)" },
    { id: "application", class: "box", x: 1370, y: 700, parent: "group", label: "Record deserializer + handler\nenrich event; write output" },
    { id: "sink", class: "box", x: 1630, y: 700, label: "Output system\nwarehouse / downstream service", fill: "#fdf2f8" },
    { id: "commit", class: "box", x: 1630, y: 590, parent: "group", label: "OffsetCommitRequest\nnext offset per partition" },
    { id: "offsets", class: "box", x: 1370, y: 285, parent: "cluster", label: "__consumer_offsets\ncompacted offsets topic" },
    { id: "retry", class: "box", x: 840, y: 330, label: "Refresh metadata\nretry retriable error" },
    { id: "callback", class: "box", x: 630, y: 330, label: "Producer callback / Future\nRecordMetadata or exception", fill: "#eff6ff" },
    { id: "retention", class: "box", x: 1630, y: 285, parent: "cluster", label: "Retention policy\nkeeps log independent of reads" },
    { id: "output-result", class: "box", x: 1810, y: 700, label: "Produced result\nconsumer side effect" },
  ] satisfies Node[],
  edges: [
    { from: "source", to: "producer", label: "record" },
    { from: "producer", to: "serializer", label: "key, value" },
    { from: "serializer", to: "partitioner", label: "serialized bytes" },
    { from: "producer", to: "bootstrap", label: "bootstrap addresses" },
    { from: "bootstrap", to: "metadata", label: "MetadataResponse\nbrokers + leaders" },
    { from: "metadata", to: "partitioner", label: "partition count / metadata" },
    { from: "partitioner", to: "accumulator", label: "TopicPartition + record" },
    { from: "accumulator", to: "sender", label: "ready batch" },
    { from: "metadata", to: "sender", label: "route to leader broker" },
    { from: "sender", to: "broker-a", label: "ProduceRequest\nclicks-0 batch" },
    { from: "sender", to: "broker-c", label: "ProduceRequest\nclicks-1 batch" },
    { from: "broker-a", to: "log-a", label: "append in order\nassign offsets" },
    { from: "broker-a", to: "replica-b", label: "replicate records" },
    { from: "log-a", to: "ack", label: "append result / offset" },
    { from: "broker-c", to: "log-c", label: "append in order\nassign offsets" },
    { from: "log-c", to: "ack", label: "append result / offset" },
    { from: "ack", to: "callback", label: "acks policy satisfied" },
    { from: "callback", to: "producer", label: "RecordMetadata or error" },
    { from: "ack", to: "retry", label: "NotLeader / retriable error" },
    { from: "retry", to: "bootstrap", label: "refresh metadata, retry" },
    { from: "coordinator", to: "assignment", label: "group membership + assignment" },
    { from: "assignment", to: "consumer-a", label: "clicks-0 ownership" },
    { from: "assignment", to: "consumer-b", label: "clicks-1 ownership" },
    { from: "consumer-a", to: "broker-a", label: "FetchRequest at offset 42" },
    { from: "consumer-b", to: "broker-c", label: "FetchRequest at offset 18" },
    { from: "log-a", to: "consumer-a", label: "FetchResponse\nrecords + high watermark" },
    { from: "log-c", to: "consumer-b", label: "FetchResponse\nrecords + high watermark" },
    { from: "consumer-a", to: "application", label: "deserialized records" },
    { from: "consumer-b", to: "application", label: "deserialized records" },
    { from: "application", to: "sink", label: "transformed event" },
    { from: "sink", to: "output-result", label: "downstream result" },
    { from: "application", to: "commit", label: "processing complete" },
    { from: "commit", to: "coordinator", label: "OffsetCommitRequest" },
    { from: "coordinator", to: "offsets", label: "append committed offsets\nreplicated before success" },
    { from: "offsets", to: "coordinator", label: "load/cache committed position" },
    { from: "coordinator", to: "commit", label: "OffsetCommitResponse" },
    { from: "log-a", to: "retention", label: "old segments expire by policy" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
