/**
 * Mastodon: https://github.com/mastodon/mastodon/blob/main/db/schema.rb
 * https://github.com/mastodon/mastodon/blob/main/app/models/status.rb
 * This slice shows how Mastodon stores accounts and authored statuses, plus
 * the join records for follows and favourites. The join tables preserve the
 * two account roles or the account/status pair and enforce uniqueness.
 */
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
    { id: "accounts", class: "box", x: 60, y: 80, label: "accounts\nid · username · uri" },
    { id: "statuses", class: "box", x: 330, y: 80, label: "statuses\nid · account_id · text · in_reply_to_id" },
    { id: "follows", class: "box", x: 60, y: 270, label: "follows\naccount_id · target_account_id\nunique pair · notify · show_reblogs" },
    { id: "favourites", class: "box", x: 330, y: 270, label: "favourites\naccount_id · status_id\nunique pair · created_at" },
    { id: "media", class: "box", x: 610, y: 80, label: "media_attachments\nid · status_id · file metadata" },
    { id: "mentions", class: "box", x: 610, y: 270, label: "mentions\naccount_id · status_id · silent\nunique pair" },
  ] satisfies Node[],
  edges: [
    { from: "accounts", to: "statuses", label: "authors · statuses.account_id" },
    { from: "statuses", to: "statuses", label: "reply · in_reply_to_id" },
    { from: "accounts", to: "follows", label: "follower · account_id" },
    { from: "follows", to: "accounts", label: "followed · target_account_id" },
    { from: "accounts", to: "favourites", label: "favouriting account_id" },
    { from: "favourites", to: "statuses", label: "favourited status_id" },
    { from: "statuses", to: "media", label: "attached · status_id" },
    { from: "accounts", to: "mentions", label: "mentioned · account_id" },
    { from: "mentions", to: "statuses", label: "in · status_id" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
