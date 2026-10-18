// GitHub Code Search Blackbird query path. Sources:
// https://github.blog/engineering/the-technology-behind-githubs-new-code-search/
// The sketch follows `/arguments?/ org:rails lang:Ruby` from the search UI through
// query rewriting, shard execution, permission filtering and the rendered results page.
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

const nodes: Node[] = [
  { id: "searcher", class: "box", x: 20, y: 180, label: "Developer\nquery" },
  { id: "github", class: "box", x: 210, y: 180, label: "GitHub.com\nsearch front end", fill: "#dbeafe" },
  { id: "query-service", class: "box", x: 420, y: 170, label: "Blackbird query service", fill: "#dcfce7", width: 190 },
  { id: "parse-rewrite", class: "box", x: 660, y: 50, label: "Parse query → AST\nrewrite scope + permissions", parent: "query-service", width: 210 },
  { id: "linguist", class: "box", x: 660, y: 290, label: "Linguist\nRuby → language ID 326", parent: "query-service", width: 190 },
  { id: "redis", class: "box", x: 660, y: 430, label: "Redis\nquotas + cached access data", parent: "query-service", width: 210 },
  { id: "fanout", class: "box", x: 920, y: 170, label: "Fan out n concurrent\nrequests: one per shard", parent: "query-service", width: 200 },
  { id: "shard", class: "box", x: 1180, y: 170, label: "Blackbird index shard\n(each shard receives query)", fill: "#fef3c7", width: 210 },
  { id: "indices", class: "box", x: 1450, y: 50, label: "Owner + language indices\ncontent / path / symbol\nngram indices", parent: "shard", width: 210 },
  { id: "iterators", class: "box", x: 1450, y: 200, label: "Lazy iterators\nAND = intersect · OR = union", parent: "shard", width: 210 },
  { id: "verify", class: "box", x: 1450, y: 360, label: "Fetch candidate documents\nverify match + ranges\nscore and sort", parent: "shard", width: 210 },
  { id: "aggregate", class: "box", x: 920, y: 570, label: "Aggregate shard results\nre-sort + recheck permissions\nreturn top 100", parent: "query-service", width: 230 },
  { id: "render", class: "box", x: 420, y: 570, label: "GitHub.com renders page\nsyntax + term highlighting\npagination", fill: "#dbeafe", width: 210 },
  { id: "answer", class: "box", x: 210, y: 570, label: "Search results\nreturned to developer" },
];

const sketch = {
  nodes,
  edges: [
    { from: "searcher", to: "github", label: "/arguments?/ org:rails lang:Ruby" },
    { from: "github", to: "query-service", label: "query" },
    { from: "query-service", to: "parse-rewrite", label: "parse" },
    { from: "parse-rewrite", to: "linguist", label: "resolve lang:Ruby" },
    { from: "linguist", to: "parse-rewrite", label: "canonical ID 326" },
    { from: "parse-rewrite", to: "redis", label: "quota + access data" },
    { from: "redis", to: "parse-rewrite", label: "quota / cached access" },
    { from: "parse-rewrite", to: "fanout", label: "rewritten AST + scope clauses" },
    { from: "fanout", to: "shard", label: "same query to each shard" },
    { from: "shard", to: "indices", label: "owners, language, ngrams" },
    { from: "indices", to: "iterators", label: "posting-list iterators" },
    { from: "iterators", to: "verify", label: "candidate document IDs" },
    { from: "verify", to: "aggregate", label: "verified, scored results" },
    { from: "aggregate", to: "render", label: "top 100" },
    { from: "render", to: "answer", label: "highlighted page" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
