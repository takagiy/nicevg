// OpenStreetMap's main database stores editable map primitives, their tags and
// ordered membership rows, alongside changesets and the users who made them.
// This sketch shows a contributor-facing slice of that schema, including how
// ways are assembled from nodes and relations group nodes, ways, or relations.
// Sources: https://wiki.openstreetmap.org/wiki/Database
// https://github.com/openstreetmap/openstreetmap-website/blob/master/db/structure.sql
// https://osm2pgsql.org/contribute/how-osm2pgsql-processing-works.html

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type TableNode = {
  id: string;
  class: "table";
  x: number;
  y: number;
  label: string;
  parent: string;
  fill?: string;
};

type GroupNode = {
  id: string;
  class: "group";
  x: number;
  y: number;
  label: string;
  width: number;
  height: number;
  fill: string;
};

type Node = TableNode | GroupNode;

const sketch = {
  nodes: [
    { id: "map-data", class: "group", x: 20, y: 20, width: 770, height: 640, label: "Current map data · PostgreSQL\nTags are key/value rows; membership rows preserve order", fill: "#f0f9ff" },
    { id: "edit-records", class: "group", x: 820, y: 20, width: 400, height: 640, label: "Edit provenance", fill: "#fff7ed" },
    { id: "current_nodes", class: "table", x: 60, y: 90, parent: "map-data", label: "current_nodes\nPK id · latitude · longitude\nchangeset_id · version · timestamp" },
    { id: "current_node_tags", class: "table", x: 60, y: 300, parent: "map-data", label: "current_node_tags\nnode_id · k · v\nPK (node_id, k)" },
    { id: "current_ways", class: "table", x: 340, y: 90, parent: "map-data", label: "current_ways\nPK id · changeset_id\nversion · timestamp" },
    { id: "current_way_nodes", class: "table", x: 340, y: 300, parent: "map-data", label: "current_way_nodes\nway_id · node_id · sequence_id\nPK (way_id, sequence_id)" },
    { id: "current_way_tags", class: "table", x: 340, y: 490, parent: "map-data", label: "current_way_tags\nway_id · k · v\nPK (way_id, k)" },
    { id: "current_relations", class: "table", x: 620, y: 90, parent: "map-data", label: "current_relations\nPK id · changeset_id\nversion · timestamp" },
    { id: "current_relation_members", class: "table", x: 620, y: 300, parent: "map-data", label: "current_relation_members\nrelation_id · member_type · member_id\nmember_role · sequence_id" },
    { id: "current_relation_tags", class: "table", x: 620, y: 490, parent: "map-data", label: "current_relation_tags\nrelation_id · k · v\nPK (relation_id, k)" },
    { id: "users", class: "table", x: 860, y: 100, parent: "edit-records", label: "users\nPK id · display_name\nemail · status" },
    { id: "changesets", class: "table", x: 860, y: 310, parent: "edit-records", label: "changesets\nPK id · user_id · created_at\nclosed_at · min/max lat/lon · num_changes" },
    { id: "changeset_tags", class: "table", x: 860, y: 510, parent: "edit-records", label: "changeset_tags\nchangeset_id · k · v\nPK (changeset_id, k)" },
  ] satisfies Node[],
  edges: [
    { from: "current_node_tags", to: "current_nodes", label: "node_id" },
    { from: "current_way_nodes", to: "current_ways", label: "way_id" },
    { from: "current_way_nodes", to: "current_nodes", label: "node_id · ordered by sequence_id" },
    { from: "current_way_tags", to: "current_ways", label: "way_id" },
    { from: "current_relation_members", to: "current_relations", label: "relation_id" },
    { from: "current_relation_members", to: "current_nodes", label: "member_type=node, member_id" },
    { from: "current_relation_members", to: "current_ways", label: "member_type=way, member_id" },
    { from: "current_relation_members", to: "current_relations", label: "member_type=relation, member_id" },
    { from: "current_relation_tags", to: "current_relations", label: "relation_id" },
    { from: "current_nodes", to: "changesets", label: "changeset_id" },
    { from: "current_ways", to: "changesets", label: "changeset_id" },
    { from: "current_relations", to: "changesets", label: "changeset_id" },
    { from: "changesets", to: "users", label: "user_id" },
    { from: "changeset_tags", to: "changesets", label: "changeset_id" },
  ],
  style: { table: box, group: box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
