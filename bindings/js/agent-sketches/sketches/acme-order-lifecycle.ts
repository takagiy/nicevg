// ACME (RFC 8555) — https://www.rfc-editor.org/rfc/rfc8555
// Shows the Order resource and the Authorization/Challenge records it depends on,
// from newOrder through identifier validation and certificate download.

import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Node = SketchNode & { class: "record" | "client" | "system"; title: string; body: string };

const record = ({ title, body }: Node) => {
  const head = text(title, 13), detail = text(body, 11);
  const w = Math.max(head.width, detail.width) + 24;
  const h = head.height + detail.height + 24;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="3" fill="#fff" stroke="#334155"/><path d="M 0 ${head.height + 16} H ${w}" stroke="#94a3b8"/><rect width="${w}" height="${head.height + 16}" rx="3" fill="#dbeafe"/><rect y="${head.height + 12}" width="${w}" height="4" fill="#dbeafe"/>${head.at({ cx: w / 2, cy: head.height / 2 + 8 })}${detail.at({ cx: w / 2, top: head.height + 20 })}`;
};

const client = ({ title, body }: Node) => {
  const a = text(title, 13), b = text(body, 11);
  const w = Math.max(a.width, b.width) + 28, h = a.height + b.height + 24;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="${h / 2}" fill="#fef3c7" stroke="#92400e"/>${a.at({ cx: w / 2, top: 8 })}${b.at({ cx: w / 2, top: a.height + 14 })}`;
};

const system = ({ title, body }: Node) => {
  const a = text(title, 13), b = text(body, 11);
  const w = Math.max(a.width, b.width) + 24, h = a.height + b.height + 24;
  return svg`<path data-bounds="true" d="M 12 0 H ${w - 12} L ${w} ${h / 2} L ${w - 12} ${h} H 12 L 0 ${h / 2} Z" fill="#dcfce7" stroke="#166534"/>${a.at({ cx: w / 2, top: 8 })}${b.at({ cx: w / 2, top: a.height + 14 })}`;
};

const nodes = [
  { id: "client", class: "client", x: 30, y: 170, title: "ACME client", body: "POST newOrder\nidentifiers: DNS names" },
  { id: "pending", class: "record", x: 290, y: 150, title: "Order · pending", body: "status\nidentifiers[]\nauthorizations[]\nexpires, finalize URL" },
  { id: "auth_pending", class: "record", x: 580, y: 40, title: "Authorization · pending", body: "identifier: dns / example.org\nstatus, expires\nchallenges[]" },
  { id: "challenge", class: "record", x: 880, y: 30, title: "Challenge · pending", body: "type, url, token\nstatus, validated\nerror?" },
  { id: "http", class: "system", x: 1150, y: 0, title: "HTTP-01 responder", body: "GET /.well-known/acme-challenge/{token}\nkey authorization" },
  { id: "dns", class: "system", x: 1150, y: 160, title: "DNS-01 responder", body: "TXT _acme-challenge.example.org\n digest of key authorization" },
  { id: "validator", class: "system", x: 1450, y: 90, title: "CA validation service", body: "fetch HTTP-01 or query DNS-01\nchecks identifier control" },
  { id: "challenge_valid", class: "record", x: 1740, y: 30, title: "Challenge · valid", body: "status=valid\nvalidated timestamp" },
  { id: "auth_valid", class: "record", x: 1740, y: 210, title: "Authorization · valid", body: "status=valid\nidentifier authorization satisfied" },
  { id: "ready", class: "record", x: 2050, y: 190, title: "Order · ready", body: "all authorizations valid\nawaiting CSR at finalize URL" },
  { id: "finalize", class: "client", x: 2050, y: 410, title: "ACME client", body: "POST finalize\nCSR identifiers must match order" },
  { id: "processing", class: "record", x: 2350, y: 190, title: "Order · processing", body: "certificate issuance underway\nRetry-After may guide polling" },
  { id: "valid", class: "record", x: 2650, y: 120, title: "Order · valid", body: "certificate URL populated\nPOST-as-GET to download" },
  { id: "invalid", class: "record", x: 2050, y: 0, title: "Order · invalid", body: "authorization fails or expires\ncertificate will not issue" },
] as const satisfies readonly Node[];

const sketch = {
  nodes,
  edges: [
    { from: "client", to: "pending", label: "newOrder response: Order URL + required authorizations" },
    { from: "pending", to: "auth_pending", label: "authorizations[] URL" },
    { from: "auth_pending", to: "challenge", label: "challenges[] URL" },
    { from: "client", to: "challenge", label: "POST challenge response" },
    { from: "challenge", to: "http", label: "publish token response" },
    { from: "challenge", to: "dns", label: "publish TXT key authorization digest" },
    { from: "http", to: "validator", label: "HTTP GET on port 80" },
    { from: "dns", to: "validator", label: "DNS TXT lookup" },
    { from: "validator", to: "challenge_valid", label: "validation succeeds" },
    { from: "validator", to: "invalid", label: "any required authorization fails" },
    { from: "challenge_valid", to: "auth_valid", label: "server marks authorization valid" },
    { from: "auth_valid", to: "ready", label: "all Order authorizations valid" },
    { from: "pending", to: "invalid", label: "Order expires before requirements met" },
    { from: "ready", to: "finalize", label: "submit PKCS#10 CSR" },
    { from: "finalize", to: "processing", label: "accepted finalize request" },
    { from: "finalize", to: "ready", label: "badCSR: amend CSR and retry" },
    { from: "processing", to: "valid", label: "CA issues certificate; certificate URL set" },
    { from: "valid", to: "client", label: "POST-as-GET certificate URL" },
  ],
  style: { record, client, system },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
