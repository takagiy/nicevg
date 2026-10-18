// Redis Cluster. Sources:
// https://redis.io/docs/latest/operate/oss_and_stack/reference/cluster-spec/
// https://github.com/redis/redis/blob/unstable/src/cluster.c
// Shows how a cluster-aware client chooses a node for one key, and how the current slot owner
// decides whether to serve, return MOVED, or use ASK while that slot is being migrated.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "app", class: "box", x: 30, y: 210, label: "Application\nGET {tenant:42}:profile" },
    { id: "client", class: "box", x: 270, y: 190, label: "Cluster-aware Redis client\ncommand + local slot map" },
    { id: "key", class: "box", x: 520, y: 55, label: "Key argument\n{tenant:42}:profile" },
    { id: "tag", class: "box", x: 770, y: 55, label: "Hash tag extraction\nfirst non-empty {...}\n`tenant:42`" },
    { id: "crc", class: "box", x: 1015, y: 55, label: "CRC16(key bytes)\nRedis Cluster hash" },
    { id: "slot", class: "box", x: 1260, y: 55, label: "Hash slot\nCRC16 & 0x3FFF\n0..16383" },
    { id: "slotmap", class: "box", x: 1515, y: 55, label: "Client slot map\nslot → endpoint:port\nmay be stale" },
    { id: "node_a", class: "box", x: 1780, y: 190, label: "Node A\nclient's selected endpoint\nmay be old owner" },
    { id: "request", class: "box", x: 2050, y: 190, label: "Command request\nGET key" },
    { id: "extract", class: "box", x: 2290, y: 190, label: "Server command key specs\nidentify key arguments" },
    { id: "server_slot", class: "box", x: 2540, y: 190, label: "getKeySlot(key)\ncompute this key's slot" },
    { id: "owner_map", class: "box", x: 2780, y: 190, label: "Node A cluster slot map\nslot → cluster node" },
    { id: "owned", class: "box", x: 3040, y: 190, label: "Is this slot served here?" },
    { id: "serve", class: "box", x: 3290, y: 35, label: "Serve command locally\nkey read / write proceeds" },
    { id: "moved", class: "box", x: 3290, y: 190, label: "MOVED slot endpoint:port\nslot owner changed" },
    { id: "refresh", class: "box", x: 3550, y: 190, label: "Client updates slot map\n(or refreshes CLUSTER SHARDS)" },
    { id: "retry", class: "box", x: 3810, y: 190, label: "Retry command\nat advertised endpoint" },
    { id: "migrating", class: "box", x: 3050, y: 440, label: "Node A slot state\nMIGRATING to Node B" },
    { id: "exists", class: "box", x: 3300, y: 440, label: "Does requested key\nexist on Node A?" },
    { id: "serve_old", class: "box", x: 3560, y: 375, label: "Serve existing key\non Node A" },
    { id: "ask", class: "box", x: 3560, y: 535, label: "ASK slot Node B\nmissing key during migration" },
    { id: "ask_client", class: "box", x: 3810, y: 535, label: "ASK handling\nredirect only this command\nkeep normal slot map" },
    { id: "asking", class: "box", x: 4060, y: 535, label: "Send ASKING\nthen redirected command" },
    { id: "node_b", class: "box", x: 4310, y: 430, label: "Node B\nslot state IMPORTING\nfrom Node A" },
    { id: "asking_flag", class: "box", x: 4560, y: 430, label: "Client ASKING flag\none command only" },
    { id: "accept", class: "box", x: 4810, y: 355, label: "Accept importing slot\nwhen ASKING was sent" },
    { id: "reject", class: "box", x: 4810, y: 535, label: "No ASKING flag\nMOVED to real owner" },
    { id: "migration_tool", class: "box", x: 3300, y: 760, label: "redis-cli resharding\nCLUSTER GETKEYSINSLOT" },
    { id: "keys_in_slot", class: "box", x: 3560, y: 760, label: "Keys returned\nrequested slot + count" },
    { id: "migrate", class: "box", x: 3810, y: 760, label: "MIGRATE Node A → Node B\nserialized key; atomic transfer" },
    { id: "normal", class: "box", x: 4310, y: 760, label: "CLUSTER SETSLOT slot NODE B\nnormal ownership after transfer" },
    { id: "client_done", class: "box", x: 4810, y: 760, label: "Command response\nreturned to application" },
  ],
  edges: [
    { from: "app", to: "client", label: "command + key" },
    { from: "client", to: "key", label: "key argument" },
    { from: "key", to: "tag", label: "key bytes" },
    { from: "tag", to: "crc", label: "tag bytes; whole key if no non-empty tag" },
    { from: "crc", to: "slot", label: "16-bit CRC result" },
    { from: "slot", to: "slotmap", label: "slot number" },
    { from: "slotmap", to: "node_a", label: "cached endpoint:port" },
    { from: "client", to: "request", label: "command + arguments" },
    { from: "node_a", to: "request", label: "send over client connection" },
    { from: "request", to: "extract", label: "argv + command key specs" },
    { from: "extract", to: "server_slot", label: "key object" },
    { from: "server_slot", to: "owner_map", label: "slot number" },
    { from: "owner_map", to: "owned", label: "owner / local slot state" },
    { from: "owned", to: "serve", label: "local node owns slot" },
    { from: "owned", to: "moved", label: "another node owns slot" },
    { from: "moved", to: "refresh", label: "MOVED error: slot + endpoint" },
    { from: "refresh", to: "retry", label: "updated slot map + original command" },
    { from: "retry", to: "node_b", label: "request to new endpoint" },
    { from: "owned", to: "migrating", label: "local slot is migrating" },
    { from: "migrating", to: "exists", label: "key for this slot" },
    { from: "exists", to: "serve_old", label: "key exists on A" },
    { from: "exists", to: "ask", label: "key missing on A" },
    { from: "serve_old", to: "client_done", label: "command result" },
    { from: "ask", to: "ask_client", label: "ASK error: slot + Node B endpoint" },
    { from: "ask_client", to: "asking", label: "same command; do not change slot map" },
    { from: "asking", to: "node_b", label: "ASKING, then original command" },
    { from: "node_b", to: "asking_flag", label: "command for IMPORTING slot" },
    { from: "asking_flag", to: "accept", label: "one-time flag present" },
    { from: "asking_flag", to: "reject", label: "flag absent / consumed" },
    { from: "accept", to: "client_done", label: "command result" },
    { from: "reject", to: "refresh", label: "MOVED slot + real owner" },
    { from: "migration_tool", to: "keys_in_slot", label: "slot, count" },
    { from: "keys_in_slot", to: "migrate", label: "key names in the slot" },
    { from: "migrate", to: "normal", label: "OK after serialized key transfer" },
    { from: "normal", to: "slotmap", label: "new owner propagates; clients refresh" },
    { from: "serve", to: "client_done", label: "command result" },
    { from: "client_done", to: "app", label: "RESP reply" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
