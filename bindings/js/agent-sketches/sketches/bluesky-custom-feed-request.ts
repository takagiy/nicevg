// Bluesky custom-feed request path, based on https://atproto.com/guides/feeds,
// https://atproto.com/guides/custom-feed-tutorial, and
// https://github.com/bluesky-social/bsky-docs/blob/main/docs/starter-templates/custom-feeds.mdx.
// The AppView resolves a declared feed to its generator, requests a URI skeleton, hydrates it,
// and returns full feed views to the app. The generator's index is populated from the repo firehose.

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
    { id: "person", class: "box", x: 20, y: 240, label: "Person\nopens a custom feed" },
    { id: "app", class: "box", x: 250, y: 240, label: "Bluesky app\nselected feed AT-URI" },
    { id: "appview", class: "box", x: 500, y: 240, label: "AppView\nfeed request coordinator" },
    { id: "feed-record", class: "box", x: 760, y: 40, label: "Feed generator record\nat://…/app.bsky.feed.generator/…" },
    { id: "generator-did", class: "box", x: 760, y: 200, label: "Generator DID document\nservice endpoint" },
    { id: "service-auth", class: "box", x: 1010, y: 200, label: "Service auth JWT\nsigned with requester repo key" },
    { id: "feedgen", class: "box", x: 1260, y: 240, label: "Feed Generator\nXRPC getFeedSkeleton" },
    { id: "index", class: "box", x: 1260, y: 500, label: "Feed index\npost URIs + ranking data" },
    { id: "algorithm", class: "box", x: 1510, y: 240, label: "Feed algorithm\nselect page by feed, limit, cursor" },
    { id: "skeleton", class: "box", x: 1760, y: 240, label: "Skeleton response\npost URIs + optional reason + cursor" },
    { id: "hydrate", class: "box", x: 1510, y: 40, label: "AppView hydration\nposts, authors, aggregates" },
    { id: "records", class: "box", x: 1260, y: 40, label: "Indexed AT records\npost content + author data" },
    { id: "feed-view", class: "box", x: 1010, y: 500, label: "Hydrated feed view\nfeed items + next cursor" },
  ],
  edges: [
    { from: "person", to: "app", label: "open feed" },
    { from: "app", to: "appview", label: "feed AT-URI, cursor" },
    { from: "appview", to: "feed-record", label: "resolve AT-URI" },
    { from: "feed-record", to: "generator-did", label: "service DID" },
    { from: "generator-did", to: "feedgen", label: "service endpoint" },
    { from: "appview", to: "service-auth", label: "requester identity" },
    { from: "service-auth", to: "feedgen", label: "Bearer JWT" },
    { from: "feedgen", to: "algorithm", label: "feed, limit, cursor" },
    { from: "index", to: "algorithm", label: "matching / ranked posts" },
    { from: "algorithm", to: "skeleton", label: "SkeletonFeedPost[]" },
    { from: "skeleton", to: "hydrate", label: "post URIs + cursor" },
    { from: "records", to: "hydrate", label: "post and author records" },
    { from: "hydrate", to: "feed-view", label: "resolved feed items" },
    { from: "feed-view", to: "appview", label: "hydrated response" },
    { from: "appview", to: "app", label: "feed view + cursor" },
    { from: "app", to: "person", label: "render feed" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
