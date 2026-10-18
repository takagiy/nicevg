import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

// https://nginx.org/en/docs/http/request_processing.html
// https://nginx.org/en/docs/http/ngx_http_core_module.html#location
// https://nginx.org/en/docs/dev/development_guide.html
// https://github.com/nginx/nginx/blob/master/src/http/ngx_http_core_module.c
// Shows NGINX's HTTP server and location decision for one request: normalized URI
// matching, prefix/regex/predicate precedence, rewrite re-entry, and content dispatch.

type Kind = "request" | "module" | "choice" | "result";
type DiagramNode = SketchNode & { class: Kind; label: string; fill?: string };

const palette: Record<Kind, string> = {
  request: "#dbeafe",
  module: "#e0e7ff",
  choice: "#fef3c7",
  result: "#dcfce7",
};

function draw(node: DiagramNode) {
  const caption = text(node.label, 12);
  const padX = 12;
  const padY = 9;
  const w = caption.width + padX * 2;
  const h = caption.height + padY * 2;
  const fill = node.fill ?? palette[node.class];
  if (node.class === "choice") {
    return svg`<path data-bounds="true" d="M ${w / 2} 0 L ${w} ${h / 2} L ${w / 2} ${h} L 0 ${h / 2} Z" fill="${fill}" stroke="#475569"/>${caption.at({ cx: w / 2, cy: h / 2 })}`;
  }
  if (node.class === "request") {
    return svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="${fill}" stroke="#475569"/>${caption.at({ cx: w / 2, cy: h / 2 })}`;
  }
  if (node.class === "result") {
    return svg`<path data-bounds="true" d="M 8 0 H ${w - 8} L ${w} ${h / 2} L ${w - 8} ${h} H 8 L 0 ${h / 2} Z" fill="${fill}" stroke="#475569"/>${caption.at({ cx: w / 2, cy: h / 2 })}`;
  }
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="5" fill="${fill}" stroke="#475569"/>${caption.at({ cx: w / 2, cy: h / 2 })}`;
}

export default {
  nodes: [
    { id: "request", class: "request", x: 20, y: 270, label: "HTTP request\nlisten address · Host · request-target" },
    { id: "listen", class: "module", x: 280, y: 270, label: "ngx_http_core_module\nlisten address / port" },
    { id: "server-name", class: "choice", x: 550, y: 260, label: "server_name lookup\nexact → wildcard → regex" },
    { id: "server", class: "result", x: 840, y: 270, label: "Selected server\nmatched name or default_server" },
    { id: "normalize", class: "module", x: 1120, y: 270, label: "Normalize request URI\n%XX decode · dot segments · slash merge" },
    { id: "uri-only", class: "result", x: 1430, y: 270, label: "Location key\nnormalized URI; query args excluded" },
    { id: "find-config", class: "module", x: 1720, y: 270, label: "NGX_HTTP_FIND_CONFIG_PHASE\nngx_http_core_find_location()" },
    { id: "exact", class: "choice", x: 2040, y: 20, label: "Exact `=` location?" },
    { id: "exact-result", class: "result", x: 2340, y: 20, label: "Use exact location\nstop search" },
    { id: "prefix-tree", class: "module", x: 2040, y: 180, label: "ngx_http_core_find_static_location()\nlocation tree: prefix comparisons" },
    { id: "best-prefix", class: "result", x: 2370, y: 180, label: "Longest matching prefix\nremember as candidate" },
    { id: "caret-tilde", class: "choice", x: 2670, y: 170, label: "Candidate has `^~`?" },
    { id: "skip-regex", class: "result", x: 2970, y: 80, label: "Keep prefix candidate\nskip regex and predicate checks" },
    { id: "regex-list", class: "module", x: 2970, y: 250, label: "Regex locations\nconfiguration order" },
    { id: "regex-hit", class: "choice", x: 3280, y: 240, label: "First regex match?" },
    { id: "regex-location", class: "result", x: 3570, y: 160, label: "Use matching regex location\nfirst match stops search" },
    { id: "predicate-list", class: "module", x: 3570, y: 350, label: "Predicate locations\nconfiguration order" },
    { id: "predicate-hit", class: "choice", x: 3870, y: 340, label: "First variable\nnonempty and nonzero?" },
    { id: "predicate-location", class: "result", x: 4180, y: 270, label: "Use predicate location" },
    { id: "prefix-fallback", class: "result", x: 4180, y: 430, label: "No later match\nuse remembered prefix" },
    { id: "location", class: "result", x: 4480, y: 270, label: "Selected location config" },
    { id: "server-rewrite", class: "module", x: 1140, y: 520, label: "ngx_http_rewrite_module\nserver rewrite phase" },
    { id: "location-rewrite", class: "module", x: 1740, y: 520, label: "ngx_http_rewrite_module\nselected location rewrite rules" },
    { id: "post-rewrite", class: "module", x: 2110, y: 520, label: "NGX_HTTP_POST_REWRITE_PHASE" },
    { id: "uri-changed", class: "choice", x: 2430, y: 520, label: "Rewrite changed URI?" },
    { id: "index", class: "module", x: 2740, y: 520, label: "ngx_http_index_module\nindex directive checks files" },
    { id: "index-found", class: "choice", x: 3040, y: 520, label: "Index file found?" },
    { id: "internal-redirect", class: "module", x: 3340, y: 650, label: "Internal redirect\nnew URI re-enters FIND_CONFIG" },
    { id: "content", class: "module", x: 3640, y: 520, label: "NGX_HTTP_CONTENT_PHASE\nlocation handler or phase handlers" },
    { id: "static", class: "result", x: 3960, y: 500, label: "ngx_http_static_module\nserve file" },
    { id: "fastcgi", class: "result", x: 3960, y: 640, label: "ngx_http_fastcgi_module\npass request to FastCGI" },
  ] satisfies DiagramNode[],
  edges: [
    { from: "request", to: "listen", label: "local socket" },
    { from: "listen", to: "server-name", label: "Host / server names" },
    { from: "server-name", to: "server", label: "match; else default" },
    { from: "server", to: "normalize", label: "request URI" },
    { from: "normalize", to: "uri-only" },
    { from: "uri-only", to: "find-config" },
    { from: "find-config", to: "exact" },
    { from: "exact", to: "exact-result", label: "yes" },
    { from: "exact-result", to: "location" },
    { from: "exact", to: "prefix-tree", label: "no" },
    { from: "prefix-tree", to: "best-prefix" },
    { from: "best-prefix", to: "caret-tilde" },
    { from: "caret-tilde", to: "skip-regex", label: "yes" },
    { from: "skip-regex", to: "location" },
    { from: "caret-tilde", to: "regex-list", label: "no" },
    { from: "regex-list", to: "regex-hit" },
    { from: "regex-hit", to: "regex-location", label: "yes" },
    { from: "regex-location", to: "location" },
    { from: "regex-hit", to: "predicate-list", label: "no regex" },
    { from: "predicate-list", to: "predicate-hit" },
    { from: "predicate-hit", to: "predicate-location", label: "yes" },
    { from: "predicate-location", to: "location" },
    { from: "predicate-hit", to: "prefix-fallback", label: "none" },
    { from: "prefix-fallback", to: "location" },
    { from: "server", to: "server-rewrite", label: "server-level rules" },
    { from: "server-rewrite", to: "find-config", label: "changed URI → find again" },
    { from: "location", to: "location-rewrite" },
    { from: "location-rewrite", to: "post-rewrite" },
    { from: "post-rewrite", to: "uri-changed" },
    { from: "uri-changed", to: "find-config", label: "yes: search again" },
    { from: "uri-changed", to: "index", label: "no" },
    { from: "index", to: "index-found" },
    { from: "index-found", to: "internal-redirect", label: "index file; new URI" },
    { from: "internal-redirect", to: "find-config" },
    { from: "index-found", to: "content", label: "no index redirect" },
    { from: "content", to: "static", label: "static location" },
    { from: "content", to: "fastcgi", label: "FastCGI location" },
  ],
  style: { request: draw, module: draw, choice: draw, result: draw },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<DiagramNode>;
