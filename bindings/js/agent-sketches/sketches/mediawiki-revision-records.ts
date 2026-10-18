// MediaWiki's page and revision storage, based on the official database
// layout documentation. This shows page history, editor attribution and the
// multi-content slot path from a revision to its content metadata and blob.
// Sources: https://www.mediawiki.org/wiki/Manual:Page_table/en
// https://www.mediawiki.org/wiki/Manual:Revision_table/en
// https://www.mediawiki.org/wiki/Manual:Actor_table
// https://www.mediawiki.org/wiki/Manual:Slots_table
// https://www.mediawiki.org/wiki/Manual:Slot_roles_table
// https://www.mediawiki.org/wiki/Manual:Content_table/en
// https://www.mediawiki.org/wiki/Manual:Text_table/en

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes = [
  { id: "page", class: "box", x: 40, y: 70, label: "page\nPK page_id · namespace · title\npage_latest · page_len" },
  { id: "revision", class: "box", x: 390, y: 80, label: "revision\nPK rev_id · rev_page · rev_parent_id\nrev_actor · rev_comment_id\ntimestamp · minor · deleted · len · sha1" },
  { id: "actor", class: "box", x: 760, y: 60, label: "actor\nPK actor_id · actor_user (nullable)\nactor_name (username or IP)" },
  { id: "user", class: "box", x: 1080, y: 80, label: "user\nPK user_id · user_name" },
  { id: "comment", class: "box", x: 760, y: 260, label: "comment\nPK comment_id · comment_text\ncomment_data" },
  { id: "slots", class: "box", x: 390, y: 320, label: "slots\nPK (slot_revision_id, slot_role_id)\nslot_content_id · slot_origin" },
  { id: "slot_roles", class: "box", x: 40, y: 340, label: "slot_roles\nPK role_id · role_name\n(e.g. main)" },
  { id: "content", class: "box", x: 760, y: 430, label: "content\nPK content_id · size · sha1\ncontent_model · content_address" },
  { id: "content_models", class: "box", x: 1080, y: 440, label: "content_models\nPK model_id · model_name" },
  { id: "text", class: "box", x: 1080, y: 610, label: "text\nPK old_id · old_text\nserialized content blob" },
  { id: "revision_tag", class: "box", x: 390, y: 570, label: "change_tag\nct_rev_id · ct_tag_id" },
  { id: "tag_def", class: "box", x: 40, y: 590, label: "change_tag_def\nPK ctd_id · ctd_name" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "page", to: "revision", label: "page_latest → rev_id (current); rev_page → page_id (history)" },
    { from: "revision", to: "revision", label: "rev_parent_id → prior revision" },
    { from: "revision", to: "actor", label: "rev_actor → actor_id" },
    { from: "actor", to: "user", label: "actor_user → user_id; NULL for anonymous actor" },
    { from: "revision", to: "comment", label: "rev_comment_id → comment_id" },
    { from: "revision", to: "slots", label: "rev_id → slot_revision_id; one row per role" },
    { from: "slots", to: "slot_roles", label: "slot_role_id → role_id" },
    { from: "slots", to: "content", label: "slot_content_id → content_id; reusable object" },
    { from: "slots", to: "revision", label: "slot_origin → rev_id that last changed this role's content" },
    { from: "content", to: "content_models", label: "content_model → model_id" },
    { from: "content", to: "text", label: "content_address tt:<old_id> points to text.old_id" },
    { from: "revision_tag", to: "revision", label: "ct_rev_id → rev_id" },
    { from: "revision_tag", to: "tag_def", label: "ct_tag_id → ctd_id" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
