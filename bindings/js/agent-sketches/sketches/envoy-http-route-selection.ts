import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

// Envoy HTTP routing docs and implementation:
// https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/http/http_routing.html
// https://github.com/envoyproxy/envoy/blob/main/source/common/router/config_impl.cc
// https://github.com/envoyproxy/envoy/blob/main/source/common/router/router.cc
// https://github.com/envoyproxy/envoy/blob/main/api/envoy/config/route/v3/route_components.proto
// Shows one HTTP request choosing a virtual host and then the first route whose path and
// additional criteria match, including route actions, cluster resolution and no-route handling.

type Node = SketchNode & { class: "box"; label: string; fill?: string };

export default {
  nodes: [
    { id: "request", class: "box", x: 20, y: 250, label: "HTTP request\n:authority · :path · :method\nheaders + stream info", fill: "#dbeafe" },
    { id: "routeconfig", class: "box", x: 260, y: 250, label: "RouteConfiguration\nvirtual_hosts[]\nordered route table", fill: "#e0e7ff" },
    { id: "authority", class: "box", x: 500, y: 90, label: "Normalize authority\nHTTP/1 Host → :authority\nport handling", fill: "#f1f5f9" },
    { id: "domains", class: "box", x: 740, y: 90, label: "VirtualHost domain lookup\nexact → suffix wildcard\n→ prefix wildcard → *", fill: "#dcfce7" },
    { id: "vhost", class: "box", x: 990, y: 90, label: "Selected VirtualHost\ndomains + routes[]\nper-vhost settings", fill: "#dcfce7" },
    { id: "novhost", class: "box", x: 990, y: 460, label: "No virtual host\nroute lookup cannot proceed", fill: "#fee2e2" },
    { id: "routeiter", class: "box", x: 1240, y: 90, label: "VirtualHost route list\ninspect in configured order\nfirst match wins", fill: "#fef3c7" },
    { id: "pathkind", class: "box", x: 1490, y: 20, label: "Path specifier\nprefix · exact path · regex\nseparated prefix · CONNECT\nURI-template extension", fill: "#ffedd5" },
    { id: "path", class: "box", x: 1490, y: 200, label: "Path match\nquery removed as applicable\ncase sensitivity configured\noptional path-parameter stripping", fill: "#ffedd5" },
    { id: "runtime", class: "box", x: 1740, y: 20, label: "runtime_fraction\nkey/default + random sample\nfailed gate → next route", fill: "#fae8ff" },
    { id: "grpc", class: "box", x: 1740, y: 150, label: "gRPC criterion\nContent-Type application/grpc\nor application/grpc+…", fill: "#fae8ff" },
    { id: "headers", class: "box", x: 1740, y: 280, label: "HeaderMatcher[]\nall configured headers\nmatch value or presence", fill: "#fae8ff" },
    { id: "query", class: "box", x: 1990, y: 20, label: "QueryParameterMatcher[]\nall configured parameters\nfirst repeated value used", fill: "#fae8ff" },
    { id: "cookies", class: "box", x: 1990, y: 150, label: "CookieMatcher[]\nparse Cookie header\nall configured cookies", fill: "#fae8ff" },
    { id: "tls", class: "box", x: 1990, y: 280, label: "TLS context criteria\npeer cert presented?\npeer cert validated?", fill: "#fae8ff" },
    { id: "metadata", class: "box", x: 2240, y: 80, label: "Dynamic metadata matchers\nfrom StreamInfo\nall must match", fill: "#fae8ff" },
    { id: "filterstate", class: "box", x: 2240, y: 240, label: "Filter-state matchers\nfrom StreamInfo\nall must match", fill: "#fae8ff" },
    { id: "miss", class: "box", x: 1490, y: 490, label: "Candidate route fails\nany criterion false\ncontinue to next configured route", fill: "#fee2e2" },
    { id: "matched", class: "box", x: 2490, y: 150, label: "Matched RouteEntry\nroute's action + route metadata\nstop searching", fill: "#dbeafe" },
    { id: "noroute", class: "box", x: 1740, y: 500, label: "No route matched\nno_route stat + NoRouteFound\nlocal 404 RouteNotFound", fill: "#fee2e2" },
    { id: "direct", class: "box", x: 2740, y: 20, label: "Direct response\nrewrite path if configured\nsend local configured reply", fill: "#fce7f3" },
    { id: "redirect", class: "box", x: 2740, y: 150, label: "Redirect action\nconstruct redirect response\nfrom redirect config", fill: "#fce7f3" },
    { id: "clusterchoice", class: "box", x: 2740, y: 300, label: "Route cluster specifier\nstatic cluster / weighted set\ncluster_header / plugin", fill: "#e0e7ff" },
    { id: "clusterresolve", class: "box", x: 2990, y: 300, label: "Resolve clusterEntry()\nheaders + stream info + random\nweighted/plugin selection", fill: "#e0e7ff" },
    { id: "cluster", class: "box", x: 3240, y: 300, label: "ThreadLocalCluster\nlookup by selected name\nmissing cluster → local error", fill: "#dcfce7" },
    { id: "forward", class: "box", x: 3490, y: 300, label: "Upstream route established\napply route settings / rewrites\ncontinue router filter", fill: "#dcfce7" },
    { id: "error", class: "box", x: 3240, y: 470, label: "Cluster unavailable\ncluster_not_found response path", fill: "#fee2e2" },
    { id: "action", class: "box", x: 2490, y: 460, label: "Route action kind\nroute · redirect · direct response", fill: "#fef3c7" },
  ],
  edges: [
    { from: "request", to: "routeconfig", label: "headers + StreamInfo" },
    { from: "routeconfig", to: "authority", label: "select virtual host" },
    { from: "authority", to: "domains", label: "normalized host value" },
    { from: "domains", to: "vhost", label: "best domain match" },
    { from: "domains", to: "novhost", label: "no domain match" },
    { from: "vhost", to: "routeiter", label: "routes in configured order" },
    { from: "routeiter", to: "pathkind", label: "candidate RouteMatch" },
    { from: "pathkind", to: "path", label: ":path and path specifier" },
    { from: "path", to: "runtime", label: "path matched" },
    { from: "runtime", to: "grpc", label: "fraction gate passes" },
    { from: "grpc", to: "headers", label: "gRPC criterion passes" },
    { from: "headers", to: "query", label: "all headers match" },
    { from: "query", to: "cookies", label: "all query params match" },
    { from: "cookies", to: "tls", label: "all cookies match" },
    { from: "tls", to: "metadata", label: "TLS context matches" },
    { from: "metadata", to: "filterstate", label: "metadata matches" },
    { from: "filterstate", to: "matched", label: "all matchers pass" },
    { from: "pathkind", to: "miss", label: "path mismatch" },
    { from: "runtime", to: "miss", label: "fraction gate fails" },
    { from: "grpc", to: "miss", label: "not gRPC" },
    { from: "headers", to: "miss", label: "header mismatch" },
    { from: "query", to: "miss", label: "query mismatch" },
    { from: "cookies", to: "miss", label: "cookie mismatch" },
    { from: "tls", to: "miss", label: "TLS mismatch" },
    { from: "metadata", to: "miss", label: "metadata mismatch" },
    { from: "filterstate", to: "miss", label: "filter state mismatch" },
    { from: "miss", to: "routeiter", label: "next route" },
    { from: "routeiter", to: "noroute", label: "list exhausted" },
    { from: "novhost", to: "noroute", label: "router has no route" },
    { from: "matched", to: "action", label: "selected RouteEntry" },
    { from: "action", to: "direct", label: "direct_response" },
    { from: "action", to: "redirect", label: "redirect" },
    { from: "action", to: "clusterchoice", label: "route action" },
    { from: "clusterchoice", to: "clusterresolve", label: "cluster specifier" },
    { from: "clusterresolve", to: "cluster", label: "cluster name" },
    { from: "cluster", to: "forward", label: "cluster found" },
    { from: "cluster", to: "error", label: "lookup fails" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
