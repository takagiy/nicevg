import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";
import { area, table, type ErNode } from "../styles/er.ts";

// OpenAlex work, authorship and author/institution records:
// https://help.openalex.org/data/works/
// https://help.openalex.org/data/authorships/
// https://help.openalex.org/data/authors/
// https://help.openalex.org/data/
// This sketch follows a scholarly work's embedded authorships to the resolved people and
// affiliations, and shows source metadata entering OpenAlex through its works API.

const sketch = {
  nodes: [
    { id: "catalog", class: "area", x: 70, y: 40, title: "OpenAlex records", width: 760, height: 540 },
    {
      id: "works", class: "table", parent: "catalog", x: 330, y: 120,
      columns: [["id", "W…", "PK"], ["doi", "URI"], ["title", "text"], ["publication_year", "int"], ["authorships", "array"]],
    },
    {
      id: "authors", class: "table", parent: "catalog", x: 100, y: 230,
      columns: [["id", "A…", "PK"], ["display_name", "text"], ["orcid", "URI"], ["works_count", "int"]],
    },
    {
      id: "authorship", class: "table", parent: "catalog", x: 340, y: 390,
      title: "authorships (inline)",
      columns: [["author", "dehydrated"], ["raw_author_name", "text"], ["author_position", "enum"], ["is_corresponding", "bool"], ["raw_affiliation_strings", "array"], ["institutions", "array"]],
    },
    {
      id: "institutions", class: "table", parent: "catalog", x: 620, y: 280,
      columns: [["id", "I…", "PK"], ["display_name", "text"], ["ror", "URI"], ["country_code", "ISO"]],
    },
    { id: "sources", class: "box", x: 880, y: 120, label: "Crossref / DataCite\nsource metadata" },
    { id: "api", class: "box", x: 900, y: 360, label: "OpenAlex API\n/works" },
  ],
  edges: [
    { from: "works", to: "authorship", label: "authorships[] embeds" },
    { from: "authorship", to: "authors", label: "author.id resolves to person" },
    { from: "authorship", to: "institutions", label: "raw affiliation → matched IDs" },
    { from: "sources", to: "works", label: "DOI + deposited metadata" },
    { from: "works", to: "api", label: "GET /works/{id}" },
  ],
  style: { table, area, box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<ErNode | { id: string; class: "box"; x: number; y: number; label: string }>;

export default sketch;
