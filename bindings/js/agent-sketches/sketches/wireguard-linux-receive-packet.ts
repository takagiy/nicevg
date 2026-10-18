// WireGuard for Linux. Sources: https://github.com/torvalds/linux/blob/master/drivers/net/wireguard/receive.c,
// https://github.com/torvalds/linux/blob/master/drivers/net/wireguard/queueing.h, and
// https://www.wireguard.com/papers/wireguard.pdf.
// Shows a transport-data packet arriving from a peer: WireGuard finds its keypair, authenticates and
// decrypts it, checks replay state and the inner source against AllowedIPs, then delivers it to wg0.
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
    { id: "udp", class: "box", x: 30, y: 150, label: "UDP socket\nWireGuard transport datagram" },
    { id: "receive", class: "box", x: 285, y: 150, label: "wg_packet_receive\nmessage type + receiver index" },
    { id: "decrypt", class: "box", x: 570, y: 150, label: "wg_packet_decrypt_worker\npeer keypair; authenticate + decrypt" },
    { id: "source-check", class: "box", x: 880, y: 150, label: "AllowedIPs lookup\nvalidate inner source IP for peer" },
    { id: "wg-rx", class: "box", x: 1170, y: 150, label: "wg0 receive queue\nskb enters virtual device" },
    { id: "ip-stack", class: "box", x: 1460, y: 150, label: "Linux IP receive path\ninner packet delivered to host" },
    { id: "drop", class: "box", x: 880, y: 390, label: "drop packet\nbad type / key / tag / counter / source" },
  ],
  edges: [
    { from: "udp", to: "receive", label: "skb: encrypted transport-data message" },
    { from: "receive", to: "decrypt", label: "receiver index resolves keypair; queue skb" },
    { from: "decrypt", to: "source-check", label: "valid AEAD tag + fresh counter; plaintext inner IP packet" },
    { from: "source-check", to: "wg-rx", label: "source matches peer's most-specific AllowedIPs prefix" },
    { from: "wg-rx", to: "ip-stack", label: "decrypted skb via netif_rx" },
    { from: "receive", to: "drop", label: "not a transport-data message or no keypair" },
    { from: "decrypt", to: "drop", label: "authentication fails or replayed counter" },
    { from: "source-check", to: "drop", label: "inner source IP not allowed for peer" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
