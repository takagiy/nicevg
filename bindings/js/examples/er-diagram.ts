// An ER diagram of a social app with chat, notifications, several ways to sign in and external
// integrations, its tables grouped by area, each foreign key drawn from its column's row to the
// key it refers to: material for edge ports.
//
//   bun examples/er-diagram.ts > er-diagram.svg

import { render, svg, text } from "nicevg";

const FONT = 12;
const HEAD = 26;
const ROW = 20;
const PAD = 8;
const KEY = 26;

/** A column: its name, its type, and whether it is the primary key or a foreign key. */
type Column = [name: string, type: string, key?: "PK" | "FK"];

type Table = {
  id: string;
  class: "table";
  x: number;
  y: number;
  parent: string;
  columns: Column[];
};

type Area = {
  id: string;
  class: "area";
  x: number;
  y: number;
  title: string;
  width: number;
  height: number;
  fill: string;
};

/** A table: its name in a tinted head, then a row per column, each a port on both sides named by the column. */
const table = ({ id, columns }: Table) => {
  const name = text(id, 13);
  const names = columns.map(([column]) => text(column, FONT));
  const types = columns.map(([, type]) => text(type, FONT));
  const namesWidth = Math.max(...names.map((line) => line.width));
  const width = Math.max(name.width + 2 * PAD, KEY + namesWidth + 3 * PAD + Math.max(...types.map((line) => line.width)) + PAD);
  const height = HEAD + columns.length * ROW;
  const rowY = (index: number) => HEAD + index * ROW + ROW / 2;
  return svg`
    <rect data-bounds="true" width="${width}" height="${height}" fill="white" stroke="#334155"/>
    <rect width="${width}" height="${HEAD}" fill="#e2e8f0" stroke="#334155"/>
    ${name.at({ cx: width / 2, cy: HEAD / 2 })}
    ${columns.map(([column, , key], index) => [
      key ? text(key, 10).at({ left: PAD, cy: rowY(index) }) : "",
      names[index]!.at({ left: KEY, cy: rowY(index) }),
      types[index]!.at({ right: width - PAD, cy: rowY(index) }),
      [0, width].map((x) => svg`<circle data-port="${column}" cx="${x}" cy="${rowY(index)}" r="0"/>`),
    ])}`;
};

/** An area: a tinted frame with its title in the top left. */
const area = ({ title, width, height, fill }: Area) => svg`
  <rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#94a3b8"/>
  ${text(title, 13).at({ left: 12, top: 8 })}`;

const tables: Omit<Table, "class">[] = [
  { id: "posts", parent: "posting", x: 60, y: 60, columns: [["id", "uuid", "PK"], ["author_id", "uuid", "FK"], ["body", "text"], ["reply_to_id", "uuid", "FK"], ["created_at", "timestamptz"]] },
  { id: "comments", parent: "posting", x: 320, y: 60, columns: [["id", "uuid", "PK"], ["post_id", "uuid", "FK"], ["author_id", "uuid", "FK"], ["body", "text"]] },
  { id: "likes", parent: "posting", x: 60, y: 220, columns: [["user_id", "uuid", "FK"], ["post_id", "uuid", "FK"], ["created_at", "timestamptz"]] },
  { id: "reposts", parent: "posting", x: 320, y: 220, columns: [["user_id", "uuid", "FK"], ["post_id", "uuid", "FK"], ["created_at", "timestamptz"]] },
  { id: "post_hashtags", parent: "posting", x: 60, y: 360, columns: [["post_id", "uuid", "FK"], ["hashtag_id", "uuid", "FK"]] },
  { id: "hashtags", parent: "posting", x: 320, y: 360, columns: [["id", "uuid", "PK"], ["name", "citext"], ["post_count", "integer"]] },
  { id: "follows", parent: "posting", x: 60, y: 500, columns: [["follower_id", "uuid", "FK"], ["followee_id", "uuid", "FK"], ["created_at", "timestamptz"]] },
  { id: "users", parent: "identity", x: 660, y: 60, columns: [["id", "uuid", "PK"], ["email", "citext"], ["display_name", "text"], ["avatar_url", "text"], ["created_at", "timestamptz"]] },
  { id: "mfa_totp_secrets", parent: "identity", x: 660, y: 220, columns: [["user_id", "uuid", "FK"], ["secret", "bytea"], ["confirmed_at", "timestamptz"]] },
  { id: "email_verifications", parent: "identity", x: 660, y: 360, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["token_hash", "text"], ["expires_at", "timestamptz"]] },
  { id: "password_credentials", parent: "identity", x: 920, y: 60, columns: [["user_id", "uuid", "FK"], ["password_hash", "text"], ["updated_at", "timestamptz"]] },
  { id: "oauth_identities", parent: "identity", x: 920, y: 200, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["provider", "text"], ["subject", "text"]] },
  { id: "webauthn_credentials", parent: "identity", x: 920, y: 340, columns: [["id", "bytea", "PK"], ["user_id", "uuid", "FK"], ["public_key", "bytea"], ["sign_count", "integer"]] },
  { id: "sessions", parent: "identity", x: 920, y: 480, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["expires_at", "timestamptz"], ["ip", "inet"]] },
  { id: "conversations", parent: "chat", x: 1260, y: 60, columns: [["id", "uuid", "PK"], ["kind", "text"], ["title", "text"], ["created_at", "timestamptz"]] },
  { id: "conversation_members", parent: "chat", x: 1500, y: 60, columns: [["conversation_id", "uuid", "FK"], ["user_id", "uuid", "FK"], ["joined_at", "timestamptz"]] },
  { id: "messages", parent: "chat", x: 1260, y: 200, columns: [["id", "uuid", "PK"], ["conversation_id", "uuid", "FK"], ["sender_id", "uuid", "FK"], ["body", "text"]] },
  { id: "attachments", parent: "chat", x: 1500, y: 200, columns: [["id", "uuid", "PK"], ["message_id", "uuid", "FK"], ["url", "text"], ["mime_type", "text"]] },
  { id: "message_reactions", parent: "chat", x: 1500, y: 340, columns: [["message_id", "uuid", "FK"], ["user_id", "uuid", "FK"], ["emoji", "text"]] },
  { id: "notifications", parent: "notification", x: 1260, y: 560, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["actor_id", "uuid", "FK"], ["post_id", "uuid", "FK"], ["read_at", "timestamptz"]] },
  { id: "push_subscriptions", parent: "notification", x: 1500, y: 560, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["endpoint", "text"], ["keys", "jsonb"]] },
  { id: "integrations", parent: "integration", x: 660, y: 760, columns: [["id", "uuid", "PK"], ["user_id", "uuid", "FK"], ["provider", "text"], ["access_token", "text"], ["refresh_token", "text"]] },
  { id: "webhooks", parent: "integration", x: 920, y: 760, columns: [["id", "uuid", "PK"], ["integration_id", "uuid", "FK"], ["url", "text"], ["secret", "text"]] },
  { id: "webhook_deliveries", parent: "integration", x: 1180, y: 760, columns: [["id", "uuid", "PK"], ["webhook_id", "uuid", "FK"], ["status", "integer"], ["delivered_at", "timestamptz"]] },
];

const nodes: (Table | Area)[] = [
  { id: "posting", class: "area", x: 20, y: 20, width: 560, height: 640, title: "Posts", fill: "#f0fdf4" },
  { id: "identity", class: "area", x: 620, y: 20, width: 560, height: 640, title: "Identity", fill: "#f0f9ff" },
  { id: "chat", class: "area", x: 1220, y: 20, width: 560, height: 460, title: "Chat", fill: "#fdf4ff" },
  { id: "notification", class: "area", x: 1220, y: 520, width: 560, height: 180, title: "Notifications", fill: "#fff7ed" },
  { id: "integration", class: "area", x: 620, y: 720, width: 820, height: 210, title: "Integrations", fill: "#fefce8" },
  ...tables.map((node) => ({ ...node, class: "table" as const })),
];

/** Each foreign key, from its column to the key it refers to, as `table.column`. */
const edges = [
  { from: "password_credentials.user_id", to: "users.id" },
  { from: "oauth_identities.user_id", to: "users.id" },
  { from: "webauthn_credentials.user_id", to: "users.id" },
  { from: "mfa_totp_secrets.user_id", to: "users.id" },
  { from: "email_verifications.user_id", to: "users.id" },
  { from: "sessions.user_id", to: "users.id" },
  { from: "posts.author_id", to: "users.id" },
  { from: "posts.reply_to_id", to: "posts.id" },
  { from: "comments.post_id", to: "posts.id" },
  { from: "comments.author_id", to: "users.id" },
  { from: "likes.user_id", to: "users.id" },
  { from: "likes.post_id", to: "posts.id" },
  { from: "reposts.user_id", to: "users.id" },
  { from: "reposts.post_id", to: "posts.id" },
  { from: "post_hashtags.post_id", to: "posts.id" },
  { from: "post_hashtags.hashtag_id", to: "hashtags.id" },
  { from: "follows.follower_id", to: "users.id" },
  { from: "follows.followee_id", to: "users.id" },
  { from: "conversation_members.conversation_id", to: "conversations.id" },
  { from: "conversation_members.user_id", to: "users.id" },
  { from: "messages.conversation_id", to: "conversations.id" },
  { from: "messages.sender_id", to: "users.id" },
  { from: "attachments.message_id", to: "messages.id" },
  { from: "message_reactions.message_id", to: "messages.id" },
  { from: "message_reactions.user_id", to: "users.id" },
  { from: "notifications.user_id", to: "users.id" },
  { from: "notifications.actor_id", to: "users.id" },
  { from: "notifications.post_id", to: "posts.id" },
  { from: "push_subscriptions.user_id", to: "users.id" },
  { from: "integrations.user_id", to: "users.id" },
  { from: "webhooks.integration_id", to: "integrations.id" },
  { from: "webhook_deliveries.webhook_id", to: "webhooks.id" },
];

// shortcut: render() does not read ports yet, so each edge goes to its table; pass `edges` as they are once it does.
const node = (end: string) => end.split(".")[0]!;

const image = render({
  nodes,
  edges: edges.map((edge) => ({ from: node(edge.from), to: node(edge.to) })),
  style: { table, area },
  rootAttrs: { "font-family": "sans-serif" },
});

// A white background over the whole viewBox, first so everything draws over it.
process.stdout.write(
  String(image).replace(/(<svg[^>]*viewBox="(\S+) (\S+) (\S+) (\S+)"[^>]*>)/, '$1\n  <rect x="$2" y="$3" width="$4" height="$5" fill="white"/>'),
);
for (const issue of image.report.issues) {
  console.error(`${issue.code}: ${issue.message}`);
}
