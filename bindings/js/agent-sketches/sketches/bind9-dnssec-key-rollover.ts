// BIND 9 DNSSEC key and signing policy (KASP).
// Sources: https://bind9.readthedocs.io/en/v9.20.2/reference.html and
// https://bind9.readthedocs.io/en/v9.16.38/dnssec.inc.html
// The sketch follows a primary zone through inline signing, key state and
// signature refresh; it also shows the operator's parent-DS step for KSK rollover.

import { svg, text, type Sketch } from "nicevg";

type Node = {
  id: string; class: "record" | "config" | "process" | "outside";
  x: number; y: number; label: string; fields?: string[];
};

const record = ({ label, fields = [] }: Node) => {
  const title = text(label, 13);
  const lines = fields.map((field) => text(field, 11));
  const w = Math.max(title.width + 20, ...lines.map((line) => line.width + 20));
  const row = 17;
  const h = 28 + lines.length * row;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="5" fill="#fff" stroke="#475569"/><path d="M0 28 H${w}" stroke="#94a3b8"/>${title.at({ left: 10, cy: 14 })}${lines.map((line, i) => line.at({ left: 10, top: 31 + i * row }))}`;
};

const config = ({ label, fields = [] }: Node) => {
  const title = text(label, 13);
  const lines = fields.map((field) => text(field, 11));
  const w = Math.max(title.width + 24, ...lines.map((line) => line.width + 24));
  const h = 26 + lines.length * 17;
  return svg`<path data-bounds="true" d="M0 0 H${w - 8} L${w} 8 V${h} H0 Z" fill="#fff7ed" stroke="#c2410c"/>${title.at({ left: 10, top: 7 })}${lines.map((line, i) => line.at({ left: 10, top: 28 + i * 17 }))}`;
};

const process = ({ label }: Node) => {
  const name = text(label, 12);
  const w = name.width + 34, h = 38;
  return svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="#eff6ff" stroke="#2563eb"/>${name.at({ cx: w / 2, cy: h / 2 })}`;
};

const outside = ({ label }: Node) => {
  const name = text(label, 12);
  const w = name.width + 24, h = 40;
  return svg`<path data-bounds="true" d="M10 0 H${w - 10} L${w} 20 L${w - 10} 40 H10 L0 20 Z" fill="#f0fdf4" stroke="#15803d"/>${name.at({ cx: w / 2, cy: 20 })}`;
};

const nodes: Node[] = [
  { id: "named-conf", class: "config", x: 40, y: 30, label: "named.conf", fields: ["zone primary", "file zone.db", "dnssec-policy default", "inline-signing yes"] },
  { id: "policy", class: "config", x: 330, y: 30, label: "KASP policy", fields: ["keys: ksk / zsk / csk", "algorithm, lifetime", "signature validity", "refresh, jitter", "publish / retire safety"] },
  { id: "named", class: "process", x: 430, y: 190, label: "named" },
  { id: "zone-db", class: "record", x: 40, y: 230, label: "zone.db", fields: ["unsigned source zone", "owner, type, TTL, RDATA"] },
  { id: "dynamic-update", class: "outside", x: 40, y: 390, label: "DDNS update" },
  { id: "zone-change", class: "process", x: 280, y: 390, label: "zone update" },
  { id: "signed-zone", class: "record", x: 670, y: 300, label: "zone.db.signed", fields: ["source RRsets", "DNSKEY", "RRSIG", "NSEC / NSEC3"] },
  { id: "key-dir", class: "record", x: 740, y: 30, label: "key directory", fields: ["K*.key: public DNSKEY", "K*.private: private key", "key state metadata"] },
  { id: "keygen", class: "process", x: 710, y: 170, label: "generate key" },
  { id: "ksk", class: "record", x: 1010, y: 30, label: "KSK", fields: ["DNSKEY flags 257", "public / private material", "publish → activate → retire"] },
  { id: "zsk", class: "record", x: 1010, y: 180, label: "ZSK", fields: ["DNSKEY flags 256", "public / private material", "publish → activate → retire"] },
  { id: "csk", class: "record", x: 1010, y: 330, label: "CSK", fields: ["combined signing key", "DNSKEY flags 257", "publish → activate → retire"] },
  { id: "key-state", class: "record", x: 1010, y: 490, label: "key lifecycle", fields: ["publish, ready, active", "retire, removed", "timers from policy"] },
  { id: "signing", class: "process", x: 720, y: 480, label: "sign RRsets" },
  { id: "rrsig", class: "record", x: 440, y: 570, label: "RRSIG RRsets", fields: ["type covered", "algorithm, key tag", "inception, expiration", "signature"] },
  { id: "refresh", class: "process", x: 160, y: 570, label: "signature refresh" },
  { id: "timer", class: "record", x: 40, y: 570, label: "signature schedule", fields: ["refresh interval", "validity + jitter", "expiration deadline"] },
  { id: "dnskey", class: "record", x: 1280, y: 200, label: "DNSKEY RRset", fields: ["flags, protocol", "algorithm, public key", "KSK / ZSK / CSK"] },
  { id: "ds", class: "record", x: 1280, y: 360, label: "DS at parent", fields: ["key tag, algorithm", "digest type, digest"] },
  { id: "parent", class: "outside", x: 1530, y: 360, label: "parent zone" },
  { id: "operator", class: "outside", x: 1530, y: 510, label: "operator" },
  { id: "cds", class: "record", x: 1280, y: 510, label: "CDS / CDNSKEY", fields: ["child signals DS change", "digest / DNSKEY data"] },
  { id: "nsec", class: "record", x: 670, y: 650, label: "NSEC chain", fields: ["next owner", "type bitmap"] },
  { id: "nsec3", class: "record", x: 950, y: 650, label: "NSEC3 chain", fields: ["hashed owner", "salt, iterations", "next hash, type bitmap"] },
  { id: "query", class: "outside", x: 40, y: 760, label: "DNS resolver query" },
  { id: "response", class: "record", x: 440, y: 760, label: "signed answer", fields: ["answer RRset", "RRSIG + denial proof"] },
  { id: "validator", class: "process", x: 830, y: 780, label: "validate chain" },
  { id: "trust", class: "record", x: 1280, y: 700, label: "trust anchor", fields: ["root DNSKEY / DS", "local configured trust"] },
];

const edges = [
  { from: "named-conf", to: "named", label: "zone + policy config" },
  { from: "policy", to: "named", label: "KASP parameters" },
  { from: "zone-db", to: "named", label: "load source zone" },
  { from: "dynamic-update", to: "zone-change", label: "DNS UPDATE" },
  { from: "zone-change", to: "named", label: "changed RRsets" },
  { from: "named", to: "key-dir", label: "read / write key files" },
  { from: "named", to: "keygen", label: "missing policy key" },
  { from: "keygen", to: "key-dir", label: "public + private key" },
  { from: "key-dir", to: "ksk", label: "KSK material" },
  { from: "key-dir", to: "zsk", label: "ZSK material" },
  { from: "key-dir", to: "csk", label: "CSK material" },
  { from: "ksk", to: "key-state", label: "role + state" },
  { from: "zsk", to: "key-state", label: "role + state" },
  { from: "csk", to: "key-state", label: "role + state" },
  { from: "policy", to: "key-state", label: "lifetime + safety timers" },
  { from: "named", to: "signing", label: "unsigned RRsets" },
  { from: "key-state", to: "signing", label: "active private keys" },
  { from: "signing", to: "rrsig", label: "RRSIG records" },
  { from: "rrsig", to: "signed-zone", label: "signed RRsets" },
  { from: "signing", to: "dnskey", label: "publish DNSKEY" },
  { from: "dnskey", to: "signed-zone", label: "DNSKEY RRset" },
  { from: "policy", to: "timer", label: "validity / refresh policy" },
  { from: "timer", to: "refresh", label: "approaching expiration" },
  { from: "refresh", to: "signing", label: "resign RRsets" },
  { from: "signed-zone", to: "nsec", label: "NSEC mode" },
  { from: "signed-zone", to: "nsec3", label: "NSEC3 mode" },
  { from: "nsec", to: "signed-zone", label: "authenticated denial" },
  { from: "nsec3", to: "signed-zone", label: "authenticated denial" },
  { from: "ksk", to: "ds", label: "digest public DNSKEY" },
  { from: "csk", to: "ds", label: "digest public DNSKEY" },
  { from: "ds", to: "parent", label: "DS RRset" },
  { from: "cds", to: "parent", label: "CDS / CDNSKEY signal" },
  { from: "operator", to: "parent", label: "submit new DS" },
  { from: "key-state", to: "cds", label: "KSK rollover signal" },
  { from: "signed-zone", to: "cds", label: "publish child signaling RR" },
  { from: "query", to: "signed-zone", label: "QNAME / QTYPE" },
  { from: "signed-zone", to: "response", label: "answer + proofs" },
  { from: "response", to: "validator", label: "RRsets + signatures" },
  { from: "parent", to: "validator", label: "DS chain" },
  { from: "trust", to: "validator", label: "trust anchor" },
];

export default { nodes, edges, style: { record, config, process, outside }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
