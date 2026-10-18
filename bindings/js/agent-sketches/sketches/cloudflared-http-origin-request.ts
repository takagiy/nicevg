// Cloudflare Tunnel (`cloudflared`). Sources:
// https://github.com/cloudflare/cloudflared/blob/master/connection/quic_connection.go
// https://github.com/cloudflare/cloudflared/blob/master/proxy/proxy.go
// https://github.com/cloudflare/cloudflared/blob/master/ingress/config.go
// Shows an HTTP request arriving over a tunnel data stream, passing optional quick-tunnel
// authorization, ingress matching and middleware, then reaching its HTTP origin and returning.
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
    { id: "client", class: "box", x: 30, y: 140, label: "HTTP client\nGET app.example.test/items" },
    { id: "edge", class: "box", x: 270, y: 140, label: "Cloudflare network\nforwards request over Tunnel" },
    { id: "stream", class: "box", x: 540, y: 140, label: "TunnelConnection\nhandleDataStream" },
    { id: "proxy", class: "box", x: 820, y: 140, label: "Proxy.ProxyHTTP\noptional authorization → ingress match\n→ rule middleware" },
    { id: "origin_proxy", class: "box", x: 1120, y: 140, label: "HTTPOriginProxy\nproxyHTTPRequest" },
    { id: "origin", class: "box", x: 1410, y: 140, label: "Matched origin service\nHTTP response" },
  ],
  edges: [
    { from: "client", to: "edge", label: "HTTP request" },
    { from: "edge", to: "stream", label: "tunnel data stream" },
    { from: "stream", to: "proxy", label: "traced request + response writer" },
    { from: "proxy", to: "origin_proxy", label: "host + path matched; middleware passed" },
    { from: "origin_proxy", to: "origin", label: "proxied HTTP request" },
    { from: "origin", to: "origin_proxy", label: "origin status, headers, body" },
    { from: "origin_proxy", to: "proxy", label: "write response to tunnel writer" },
    { from: "proxy", to: "stream", label: "HTTP response" },
    { from: "stream", to: "edge", label: "response on tunnel stream" },
    { from: "edge", to: "client", label: "HTTP response" },
    { from: "proxy", to: "stream", label: "middleware filter: configured status; skip origin" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
