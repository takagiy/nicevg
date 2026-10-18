// Redis. Sources: https://redis.io/docs/latest/commands/set/ and
// https://github.com/redis/redis/blob/unstable/src/t_string.c
// Shows the server path for SET key value: RESP input is parsed and dispatched, the string command
// applies the write to the selected database, Redis propagates it, then replies to the client.
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
    { id: "client", class: "box", x: 30, y: 125, label: "Redis client\nSET bike:1 Deimos" },
    { id: "input", class: "box", x: 260, y: 125, label: "readQueryFromClient\nRESP command arguments" },
    { id: "dispatch", class: "box", x: 500, y: 125, label: "processCommand\ncommand lookup + checks" },
    { id: "set", class: "box", x: 740, y: 125, label: "setCommand\nparse options; call setGenericCommand" },
    { id: "database", class: "box", x: 1000, y: 125, label: "selected Redis DB\nkey bike:1 → string Deimos" },
    { id: "propagate", class: "box", x: 1000, y: 315, label: "propagate command\nto AOF and replicas as configured" },
    { id: "reply", class: "box", x: 500, y: 315, label: "Client reply\n+OK" },
  ],
  edges: [
    { from: "client", to: "input", label: "RESP request bytes" },
    { from: "input", to: "dispatch", label: "argv: SET, bike:1, Deimos" },
    { from: "dispatch", to: "set", label: "resolved command; invoke proc" },
    { from: "set", to: "database", label: "setKey(db, key, value)" },
    { from: "database", to: "propagate", label: "successful write" },
    { from: "propagate", to: "reply", label: "command completes" },
    { from: "reply", to: "client", label: "RESP simple string +OK" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
