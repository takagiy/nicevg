// Zephyr Project's native networking stack, following the documented BSD socket packet paths.
// It shows how RX and TX move through socket handling, IP, net_if, L2 and the device driver,
// including the FIFO handoff that separates driver context from stack processing.
// Sources: https://docs.zephyrproject.org/latest/services/connectivity/networking/net-stack-architecture.html
//          https://docs.zephyrproject.org/latest/services/connectivity/networking/api/net_if.html
//          https://docs.zephyrproject.org/latest/services/connectivity/networking/api/net_l2.html
//          https://docs.zephyrproject.org/latest/services/connectivity/networking/api/net_pkt.html
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
    { id: "application", class: "box", x: 30, y: 250, label: "Network application thread\nBSD socket send / recv" },
    { id: "socket-api", class: "box", x: 290, y: 250, label: "BSD socket API\nsocket context + protocol headers" },
    { id: "ip", class: "box", x: 570, y: 250, label: "L3 IP processing\nIPv4 / IPv6 headers and routing" },
    { id: "net-if", class: "box", x: 850, y: 250, label: "net_if\ninterface state, queues, L2 nexus" },
    { id: "tx-queue", class: "box", x: 1130, y: 100, label: "TX k_fifo\npriority-classified packets" },
    { id: "rx-queue", class: "box", x: 1130, y: 410, label: "RX k_fifo\ninterrupt-to-thread handoff" },
    { id: "l2", class: "box", x: 1410, y: 250, label: "L2 implementation\nEthernet / Wi-Fi / 802.15.4" },
    { id: "driver", class: "box", x: 1690, y: 250, label: "Network device driver\nlow-level send / receive" },
    { id: "medium", class: "box", x: 1970, y: 250, label: "Link / peer\nphysical network" },
    { id: "socket-handler", class: "box", x: 570, y: 570, label: "Socket handler\nfinds matching active socket" },
    { id: "socket-rx-queue", class: "box", x: 290, y: 570, label: "Socket receive queue\nseparates stack from app" },
    { id: "net-pkt", class: "box", x: 850, y: 570, label: "net_pkt\niface + context + cursor + fifo" },
    { id: "net-buf", class: "box", x: 1130, y: 650, label: "net_buf fragments\npacket bytes; shared by net_pkt" },
  ],
  edges: [
    { from: "application", to: "socket-api", label: "send(data)" },
    { from: "socket-api", to: "ip", label: "payload + UDP/TCP header" },
    { from: "ip", to: "net-if", label: "IP packet; select enabled interface" },
    { from: "net-if", to: "tx-queue", label: "classify and enqueue TX net_pkt" },
    { from: "tx-queue", to: "l2", label: "TX processing thread dequeues packet" },
    { from: "l2", to: "driver", label: "add L2 framing; driver send API" },
    { from: "driver", to: "medium", label: "transmit frame" },
    { from: "medium", to: "driver", label: "received frame" },
    { from: "driver", to: "rx-queue", label: "allocate net_pkt; enqueue from RX context" },
    { from: "rx-queue", to: "l2", label: "RX processing thread dequeues packet" },
    { from: "l2", to: "net-if", label: "net_recv_data(net_pkt)" },
    { from: "net-if", to: "ip", label: "pass packet to L3" },
    { from: "ip", to: "socket-handler", label: "validated IP datagram" },
    { from: "socket-handler", to: "socket-rx-queue", label: "queue for matching socket" },
    { from: "socket-rx-queue", to: "socket-api", label: "recv() obtains payload" },
    { from: "socket-api", to: "application", label: "return data to app thread" },
    { from: "net-pkt", to: "net-buf", label: "frags point to packet storage" },
    { from: "net-pkt", to: "net-if", label: "iface metadata" },
    { from: "socket-api", to: "net-pkt", label: "TX builds packet wrapper" },
    { from: "l2", to: "net-pkt", label: "RX / TX layers read and write headers" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
