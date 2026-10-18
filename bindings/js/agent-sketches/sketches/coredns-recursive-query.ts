// CoreDNS. Sources: https://github.com/coredns/coredns/blob/master/core/dnsserver/server.go,
// https://github.com/coredns/coredns/blob/master/plugin/forward/README.md, and
// https://coredns.io/manual/toc/.
// Shows a recursive A query through the UDP listener, zone selection and cache/forward plugin chain.
// The upstream answer returns through cache and the response writer to the original DNS client.
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
    { id: "client", class: "box", x: 30, y: 200, label: "DNS client\nA example.org." },
    { id: "udp", class: "box", x: 265, y: 200, label: "UDP :53 listener\nServePacket" },
    { id: "dns-server", class: "box", x: 500, y: 200, label: "miekg/dns Server\nunpack DNS message" },
    { id: "entry", class: "box", x: 745, y: 200, label: "dnsserver.Server.ServeDNS\nquestion + class + EDNS checks" },
    { id: "zone", class: "box", x: 1015, y: 200, label: "zone multiplexer\nlongest suffix match; root fallback" },
    { id: "chain", class: "box", x: 1280, y: 200, label: "Config.pluginChain\nServeDNS(ctx, writer, *dns.Msg)" },
    { id: "cache", class: "box", x: 1510, y: 90, label: "cache.ServeDNS\nlookup by question" },
    { id: "forward", class: "box", x: 1510, y: 310, label: "forward.ServeDNS\nNext on cache miss" },
    { id: "proxy", class: "box", x: 1280, y: 430, label: "forward.Proxy\npolicy selects upstream" },
    { id: "upstream", class: "box", x: 1015, y: 430, label: "upstream DNS resolver\nexchange query; return *dns.Msg" },
    { id: "cache-store", class: "box", x: 745, y: 430, label: "cache response path\nstore eligible answer + TTL" },
    { id: "writer", class: "box", x: 500, y: 430, label: "ScrubWriter / ResponseWriter\nfit reply to client's UDP buffer" },
    { id: "reply", class: "box", x: 265, y: 430, label: "DNS response\nA records or upstream RCODE" },
  ],
  edges: [
    { from: "client", to: "udp", label: "UDP DNS datagram" },
    { from: "udp", to: "dns-server", label: "packet + response writer" },
    { from: "dns-server", to: "entry", label: "decoded *dns.Msg; context" },
    { from: "entry", to: "zone", label: "lowercase question name" },
    { from: "entry", to: "writer", label: "bad / unsupported query: error RCODE" },
    { from: "zone", to: "chain", label: "selected zone Config" },
    { from: "chain", to: "cache", label: "DNS query" },
    { from: "cache", to: "writer", label: "cache hit: cached DNS answer" },
    { from: "cache", to: "forward", label: "cache miss: Next" },
    { from: "forward", to: "proxy", label: "select endpoint using policy" },
    { from: "proxy", to: "upstream", label: "DNS exchange (normally UDP)" },
    { from: "upstream", to: "cache-store", label: "upstream response message" },
    { from: "cache-store", to: "writer", label: "answer after cache insertion" },
    { from: "writer", to: "reply", label: "size-adjusted DNS response" },
    { from: "reply", to: "client", label: "UDP response datagram" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
