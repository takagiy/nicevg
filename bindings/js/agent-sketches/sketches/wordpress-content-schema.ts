// WordPress content and taxonomy records, based on the official database schema and table documentation.
// Shows the core post, comment, user, metadata, and taxonomy records and the keys WordPress uses to relate them.
// Sources: https://developer.wordpress.org/reference/functions/wp_get_db_schema/
//          https://developer.wordpress.org/reference/classes/wpdb/
//          https://developer.wordpress.org/themes/classic-themes/basics/categories-tags-custom-taxonomies/
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const sketch = {
  nodes: [
    { id: "wp_posts", class: "box", x: 360, y: 40, label: "wp_posts\nPK ID · post_author · post_date\npost_content · post_title · post_status\npost_parent · post_type · post_mime_type", width: 290, height: 96 },
    { id: "wp_users", class: "box", x: 20, y: 40, label: "wp_users\nPK ID · user_login · user_pass\nuser_email · user_registered", width: 250, height: 82 },
    { id: "wp_comments", class: "box", x: 720, y: 40, label: "wp_comments\nPK comment_ID · comment_post_ID\ncomment_author · comment_author_email\ncomment_content · comment_approved", width: 290, height: 96 },
    { id: "wp_postmeta", class: "box", x: 360, y: 190, label: "wp_postmeta\nPK meta_id · post_id · meta_key\nmeta_value", width: 250, height: 76 },
    { id: "wp_commentmeta", class: "box", x: 720, y: 190, label: "wp_commentmeta\nPK meta_id · comment_id · meta_key\nmeta_value", width: 270, height: 76 },
    { id: "wp_usermeta", class: "box", x: 20, y: 190, label: "wp_usermeta\nPK umeta_id · user_id · meta_key\nmeta_value", width: 250, height: 76 },
    { id: "wp_terms", class: "box", x: 360, y: 360, label: "wp_terms\nPK term_id · name · slug\nterm_group", width: 230, height: 76 },
    { id: "wp_term_taxonomy", class: "box", x: 680, y: 360, label: "wp_term_taxonomy\nPK term_taxonomy_id · term_id\ntaxonomy · description · parent · count", width: 290, height: 82 },
    { id: "wp_term_relationships", class: "box", x: 360, y: 520, label: "wp_term_relationships\nPK (object_id, term_taxonomy_id)\nterm_order", width: 290, height: 76 },
    { id: "wp_termmeta", class: "box", x: 20, y: 360, label: "wp_termmeta\nPK meta_id · term_id · meta_key\nmeta_value", width: 250, height: 76 },
    { id: "wp_options", class: "box", x: 20, y: 520, label: "wp_options\nPK option_id · UQ option_name\noption_value · autoload", width: 250, height: 76 },
    { id: "wp_links", class: "box", x: 720, y: 520, label: "wp_links (legacy blogroll)\nPK link_id · link_url · link_name\nlink_owner · link_visible", width: 290, height: 76 },
  ],
  edges: [
    { from: "wp_posts", to: "wp_users", label: "post_author → ID" },
    { from: "wp_posts", to: "wp_posts", label: "post_parent → ID" },
    { from: "wp_comments", to: "wp_posts", label: "comment_post_ID → ID" },
    { from: "wp_postmeta", to: "wp_posts", label: "post_id → ID" },
    { from: "wp_commentmeta", to: "wp_comments", label: "comment_id → comment_ID" },
    { from: "wp_usermeta", to: "wp_users", label: "user_id → ID" },
    { from: "wp_term_taxonomy", to: "wp_terms", label: "term_id → term_id" },
    { from: "wp_termmeta", to: "wp_terms", label: "term_id → term_id" },
    { from: "wp_term_relationships", to: "wp_term_taxonomy", label: "term_taxonomy_id" },
    { from: "wp_term_relationships", to: "wp_posts", label: "object_id → posts.ID (post object)" },
  ],
  style: { box },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string; width: number; height: number }>;

export default sketch;
