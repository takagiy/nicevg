// Drupal 11 entity storage: https://www.drupal.org/docs/drupal-apis/entity-api/content-entity
// Node storage schema: https://api.drupal.org/api/drupal/core%21modules%21node%21src%21Entity%21Node.php/class/Node/11.x
// Node access schema and hooks: https://api.drupal.org/api/drupal/core%21modules%21node%21node.install/function/node_schema/11.x
// Node grants guide: https://www.drupal.org/docs/develop/drupal-apis/writing-module-that-handles-node-access
// This sketch follows a node's revisioned fields and authorship alongside Drupal's role and node-grant records. It shows how users obtain role permissions and how matching realm/gid grants gate node queries.
import type { Sketch } from "nicevg";
import { area, table, type ErNode } from "../styles/er.ts";

const nodes: ErNode[] = [
  { id: "content", class: "area", x: 10, y: 10, title: "Content entity and revision storage", width: 880, height: 600 },
  { id: "identity", class: "area", x: 910, y: 10, title: "Account, role and permission records", width: 650, height: 600, fill: "#eff6ff" },
  { id: "grants", class: "area", x: 10, y: 630, title: "Node access grant index (realm + gid match)", width: 1550, height: 360, fill: "#fff7ed" },
  { id: "node", class: "table", x: 35, y: 55, parent: "content", columns: [["nid", "int", "PK"], ["vid", "int", "FK"], ["type", "varchar"], ["uuid", "varchar"]] },
  { id: "node_field_data", class: "table", x: 270, y: 55, parent: "content", columns: [["nid", "int", "PK FK"], ["vid", "int"], ["langcode", "varchar"], ["type", "varchar"], ["title", "varchar"], ["uid", "int", "FK"], ["status", "int"], ["created", "int"]] },
  { id: "node_revision", class: "table", x: 35, y: 255, parent: "content", columns: [["nid", "int", "PK"], ["vid", "int", "PK"], ["langcode", "varchar"], ["revision_uid", "int", "FK"], ["revision_timestamp", "int"], ["revision_log", "varchar"]] },
  { id: "node_field_revision", class: "table", x: 270, y: 255, parent: "content", columns: [["nid", "int", "PK"], ["vid", "int", "PK"], ["langcode", "varchar"], ["title", "varchar"], ["uid", "int", "FK"], ["status", "int"]] },
  { id: "node__body", class: "table", x: 545, y: 55, parent: "content", columns: [["entity_id", "int", "PK FK"], ["revision_id", "int", "PK"], ["langcode", "varchar"], ["delta", "int", "PK"], ["body_value", "longtext"], ["body_format", "varchar"]] },
  { id: "node_revision__body", class: "table", x: 545, y: 255, parent: "content", columns: [["entity_id", "int", "PK FK"], ["revision_id", "int", "PK"], ["langcode", "varchar"], ["delta", "int", "PK"], ["body_value", "longtext"]] },
  { id: "node_type", class: "table", x: 35, y: 440, parent: "content", columns: [["id", "config name", "PK"], ["label", "config value"], ["new_revision", "config value"]] },
  { id: "taxonomy_term_field_data", class: "table", x: 270, y: 440, parent: "content", columns: [["tid", "int", "PK"], ["vid", "varchar", "FK"], ["name", "varchar"], ["langcode", "varchar"]] },
  { id: "node__field_tags", class: "table", x: 545, y: 440, parent: "content", columns: [["entity_id", "int", "PK FK"], ["revision_id", "int", "PK"], ["delta", "int", "PK"], ["field_tags_target_id", "int", "FK"]] },
  { id: "taxonomy_term_data", class: "table", x: 35, y: 555, parent: "content", columns: [["tid", "int", "PK"], ["vid", "varchar", "FK"], ["uuid", "varchar"]] },
  { id: "taxonomy_term_field_revision", class: "table", x: 270, y: 555, parent: "content", columns: [["tid", "int", "PK FK"], ["revision_id", "int", "PK"], ["langcode", "varchar"], ["name", "varchar"]] },
  { id: "taxonomy_term_revision", class: "table", x: 545, y: 555, parent: "content", columns: [["tid", "int", "PK"], ["revision_id", "int", "PK"], ["revision_user", "int", "FK"], ["revision_created", "int"]] },
  { id: "taxonomy_vocabulary", class: "table", x: 745, y: 440, parent: "content", columns: [["vid", "varchar", "PK"], ["name", "varchar"], ["weight", "int"]] },
  { id: "users_field_data", class: "table", x: 935, y: 55, parent: "identity", columns: [["uid", "int", "PK"], ["langcode", "varchar"], ["name", "varchar"], ["mail", "varchar"], ["status", "int"]] },
  { id: "user__roles", class: "table", x: 1190, y: 55, parent: "identity", columns: [["entity_id", "int", "PK FK"], ["roles_target_id", "varchar", "PK FK"], ["delta", "int", "PK"]] },
  { id: "user_role", class: "table", x: 935, y: 235, parent: "identity", columns: [["id", "varchar", "PK"], ["label", "varchar"], ["weight", "int"], ["is_admin", "int"]] },
  { id: "role_permission", class: "table", x: 1190, y: 235, parent: "identity", columns: [["rid", "varchar", "PK FK"], ["permission", "varchar", "PK"], ["module", "varchar", "PK"], ["langcode", "varchar"]] },
  { id: "user__user_picture", class: "table", x: 935, y: 425, parent: "identity", columns: [["entity_id", "int", "PK FK"], ["user_picture_target_id", "int", "FK"]] },
  { id: "file_managed", class: "table", x: 1190, y: 425, parent: "identity", columns: [["fid", "int", "PK"], ["uid", "int", "FK"], ["uri", "varchar"], ["status", "int"]] },
  { id: "users", class: "table", x: 935, y: 555, parent: "identity", columns: [["uid", "int", "PK"], ["uuid", "varchar"], ["default_langcode", "varchar"]] },
  { id: "user__user_picture_revision", class: "table", x: 1190, y: 555, parent: "identity", columns: [["entity_id", "int", "PK FK"], ["revision_id", "int", "PK"], ["user_picture_target_id", "int", "FK"]] },
  { id: "node_access", class: "table", x: 55, y: 700, parent: "grants", columns: [["nid", "int", "PK FK"], ["langcode", "varchar", "PK"], ["fallback", "int", "PK"], ["realm", "varchar", "PK"], ["gid", "int", "PK"], ["grant_view", "int"], ["grant_update", "int"], ["grant_delete", "int"]] },
  { id: "account_grants", class: "table", x: 380, y: 700, parent: "grants", title: "hook_node_grants(account, op)", columns: [["realm", "issued key"], ["gid", "issued grant ID"], ["op", "view / update / delete"]] },
  { id: "access_query", class: "table", x: 745, y: 700, parent: "grants", title: "Node access query", columns: [["nid", "candidate node"], ["langcode", "query language"], ["realm + gid", "must match issued key"], ["grant_*", "operation flag >= 1"]] },
  { id: "access_api", class: "table", x: 1110, y: 700, parent: "grants", title: "NodeAccessControlHandler", columns: [["operation", "view / update / delete"], ["account", "current user"], ["permission", "e.g. bypass node access"], ["result", "allowed / forbidden"]] },
];

const edges = [
  { from: "node.vid", to: "node_revision.vid", label: "default revision id" },
  { from: "node_field_data.nid", to: "node.nid", label: "current translation data" },
  { from: "node_field_data.uid", to: "users_field_data.uid", label: "content author" },
  { from: "node_revision.nid", to: "node.nid", label: "revision history per node" },
  { from: "node_revision.revision_uid", to: "users_field_data.uid", label: "revision author" },
  { from: "node_field_revision.nid", to: "node_revision.nid", label: "revision's translated data" },
  { from: "node__body.entity_id", to: "node.nid", label: "current body field items" },
  { from: "node__body.revision_id", to: "node_revision.vid", label: "entity revision" },
  { from: "node_revision__body.entity_id", to: "node.nid", label: "revision body field items" },
  { from: "node_revision__body.revision_id", to: "node_revision.vid", label: "field values for revision" },
  { from: "node_field_data.type", to: "node_type.id", label: "bundle config entity" },
  { from: "node__field_tags.entity_id", to: "node.nid", label: "tag field on node" },
  { from: "node__field_tags.field_tags_target_id", to: "taxonomy_term_field_data.tid", label: "referenced taxonomy term" },
  { from: "taxonomy_term_field_data.tid", to: "taxonomy_term_data.tid", label: "translated term data" },
  { from: "taxonomy_term_field_revision.tid", to: "taxonomy_term_data.tid", label: "term revisions" },
  { from: "taxonomy_term_revision.tid", to: "taxonomy_term_data.tid", label: "revision metadata" },
  { from: "taxonomy_term_field_data.vid", to: "taxonomy_vocabulary.vid", label: "term belongs to vocabulary" },
  { from: "user__roles.entity_id", to: "users_field_data.uid", label: "account role membership" },
  { from: "user__roles.roles_target_id", to: "user_role.id", label: "assigned role" },
  { from: "role_permission.rid", to: "user_role.id", label: "role's permission rows" },
  { from: "user__user_picture.entity_id", to: "users_field_data.uid", label: "account picture field" },
  { from: "user__user_picture.user_picture_target_id", to: "file_managed.fid", label: "managed image file" },
  { from: "users.uid", to: "users_field_data.uid", label: "account identity/base record" },
  { from: "user__user_picture_revision.entity_id", to: "users.uid", label: "revisioned profile image field" },
  { from: "user__user_picture_revision.user_picture_target_id", to: "file_managed.fid", label: "revision image target" },
  { from: "node_access.nid", to: "node.nid", label: "grant rows for node (nid 0 = all nodes)" },
  { from: "users_field_data", to: "account_grants", label: "account + operation produce realm/gid keys" },
  { from: "role_permission", to: "account_grants", label: "permission checks decide which keys to issue" },
  { from: "account_grants", to: "access_query", label: "realm/gid keys supplied to query" },
  { from: "node_access", to: "access_query", label: "node, language and grant flags filtered" },
  { from: "access_query", to: "access_api", label: "candidate access result" },
  { from: "users_field_data", to: "access_api", label: "current account" },
];

export default { nodes, edges, style: { table, area }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<ErNode>;
