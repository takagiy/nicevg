// Certbot HTTP-01 issuance, based on https://github.com/certbot/certbot/blob/main/certbot/docs/using.rst,
// https://github.com/certbot/certbot/blob/main/certbot/src/certbot/_internal/client.py,
// https://github.com/certbot/certbot/blob/main/certbot/src/certbot/_internal/auth_handler.py,
// https://github.com/certbot/certbot/blob/main/certbot/src/certbot/_internal/plugins/standalone.py,
// https://github.com/certbot/certbot/blob/main/acme/src/acme/client.py, and
// https://letsencrypt.org/docs/challenge-types/. Follows `certbot certonly --standalone -d example.org`
// from CLI setup through HTTP-01 validation, ACME order finalization, cleanup, and saved lineage.

import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
  x: number;
  y: number;
  label: string;
  fill?: string;
  width?: number;
  height?: number;
};

const nodes: Node[] = [
  { id: "operator", class: "circle", x: 20, y: 205, label: "Operator\ncertonly --standalone\n-d example.org", fill: "#fef3c7" },
  { id: "cli", class: "box", x: 210, y: 190, label: "certbot CLI\nparse command + config", fill: "#dbeafe" },
  { id: "plugins", class: "box", x: 430, y: 80, label: "Plugin discovery\nselect standalone authenticator\nno installer for certonly", fill: "#dbeafe" },
  { id: "client", class: "box", x: 430, y: 220, label: "Certbot client\nregister account, then obtain", fill: "#dbeafe" },
  { id: "account_key", class: "box", x: 650, y: 20, label: "Account key\nJWS signing identity", fill: "#e0e7ff" },
  { id: "directory", class: "box", x: 650, y: 145, label: "ACME directory\nnewNonce / newAccount /\nnewOrder endpoint URLs", fill: "#e0e7ff" },
  { id: "account", class: "box", x: 870, y: 20, label: "ACME account\naccount URL + contact + key", fill: "#fce7f3" },
  { id: "cert_key", class: "box", x: 650, y: 300, label: "Certificate private key\nnew RSA or ECDSA key\n(or configured reuse-key)", fill: "#e0e7ff" },
  { id: "csr", class: "box", x: 870, y: 300, label: "CSR\nsubjectAltName: example.org\nsigned by certificate key", fill: "#e0e7ff" },
  { id: "new_order", class: "box", x: 1090, y: 235, label: "ClientV2.new_order\nPOST signed CSR", fill: "#dbeafe" },
  { id: "order", class: "box", x: 1310, y: 235, label: "ACME Order\npending; order URL\nauthorization URLs", fill: "#fce7f3" },
  { id: "authzs", class: "box", x: 1530, y: 235, label: "Authorization resources\nidentifier: DNS example.org\nstatus + offered challenges", fill: "#fce7f3" },
  { id: "choose", class: "box", x: 1750, y: 235, label: "AuthHandler\n_choose_challenges\nselect offered HTTP-01", fill: "#dbeafe" },
  { id: "token", class: "box", x: 1970, y: 100, label: "HTTP-01 challenge\ntoken from CA", fill: "#fce7f3" },
  { id: "keyauth", class: "box", x: 1970, y: 270, label: "response_and_validation()\nkeyAuthorization = token +\naccount-key thumbprint", fill: "#e0e7ff" },
  { id: "standalone", class: "box", x: 2190, y: 270, label: "StandaloneAuthenticator\nperform(achalls)", fill: "#dbeafe" },
  { id: "bind", class: "box", x: 2410, y: 100, label: "HTTP01DualNetworkedServers\nbind configured address:80\nIPv6 / IPv4 listeners", fill: "#e0e7ff" },
  { id: "bind_error", class: "box", x: 2410, y: 430, label: "Bind failure\npermission denied or port busy\nretry prompt / PluginError", fill: "#fee2e2" },
  { id: "resource", class: "box", x: 2630, y: 270, label: "HTTP01Resource\nchall + response + validation\nregistered in request handler", fill: "#e0e7ff" },
  { id: "ready", class: "box", x: 2850, y: 270, label: "Challenge answer\nanswer_challenge(challb, response)", fill: "#dbeafe" },
  { id: "ca_validate", class: "box", x: 3070, y: 130, label: "CA HTTP-01 validator\nfetch port 80 URL from domain\nmay check multiple vantage points", fill: "#fce7f3" },
  { id: "http_get", class: "box", x: 3290, y: 270, label: "GET /.well-known/acme-challenge/{token}\nHTTP01RequestHandler serves\nkeyAuthorization response", fill: "#e0e7ff" },
  { id: "valid", class: "box", x: 3510, y: 130, label: "Authorization valid\nCA matched response", fill: "#dcfce7" },
  { id: "invalid", class: "box", x: 3510, y: 430, label: "Authorization invalid\nconnection / wrong content / timeout\nValidationError", fill: "#fee2e2" },
  { id: "poll_auth", class: "box", x: 3730, y: 270, label: "AuthHandler polls authz\nPOST-as-GET until status changes\nmax retries / time limit", fill: "#dbeafe" },
  { id: "cleanup", class: "box", x: 3950, y: 430, label: "ExitHandler cleanup\nremove served challenge mapping\nstop listener when unused", fill: "#e0e7ff" },
  { id: "finalize", class: "box", x: 3950, y: 170, label: "ClientV2.finalize_order\nPOST CSR to finalize URL", fill: "#dbeafe" },
  { id: "issued", class: "box", x: 4170, y: 170, label: "Order ready\nCA signed certificate\nfullchain PEM", fill: "#dcfce7" },
  { id: "poll_issue", class: "box", x: 4390, y: 170, label: "Poll order / certificate URL\nuntil issued or issuance timeout", fill: "#dbeafe" },
  { id: "split", class: "box", x: 4610, y: 170, label: "cert_and_chain_from_fullchain\nleaf certificate + chain PEM", fill: "#e0e7ff" },
  { id: "lineage", class: "box", x: 4830, y: 170, label: "storage.RenewableCert.new_lineage\ncreate archive + renewal config\nupdate live symlinks", fill: "#dbeafe" },
  { id: "live", class: "box", x: 5050, y: 170, label: "/etc/letsencrypt/live/<name>/\ncert.pem · chain.pem\nfullchain.pem · privkey.pem", fill: "#dcfce7" },
  { id: "result", class: "circle", x: 5270, y: 170, label: "certonly\nreturns success", fill: "#fef3c7" },
];

const sketch = {
  nodes,
  edges: [
    { from: "operator", to: "cli", label: "command + domain" },
    { from: "cli", to: "plugins", label: "authenticator requested" },
    { from: "plugins", to: "client", label: "StandaloneAuthenticator" },
    { from: "client", to: "account_key", label: "load or create key" },
    { from: "client", to: "directory", label: "GET directory URL" },
    { from: "directory", to: "client", label: "ACME endpoint map" },
    { from: "client", to: "account", label: "newAccount; signed with account key" },
    { from: "account", to: "client", label: "account URL / registration" },
    { from: "client", to: "cert_key", label: "generate key (unless reusing)" },
    { from: "cert_key", to: "csr", label: "sign CSR" },
    { from: "client", to: "new_order", label: "requested SANs" },
    { from: "csr", to: "new_order", label: "DER CSR" },
    { from: "new_order", to: "order", label: "newOrder request" },
    { from: "order", to: "authzs", label: "POST-as-GET authorization URLs" },
    { from: "authzs", to: "choose", label: "pending authorization + challenges" },
    { from: "choose", to: "token", label: "selected HTTP-01" },
    { from: "token", to: "keyauth", label: "token + account key thumbprint" },
    { from: "keyauth", to: "standalone", label: "AnnotatedChallenge" },
    { from: "standalone", to: "bind", label: "start temporary server on port 80" },
    { from: "bind", to: "bind_error", label: "EACCES / EADDRINUSE" },
    { from: "standalone", to: "resource", label: "add HTTP01Resource" },
    { from: "resource", to: "ready", label: "challenge response" },
    { from: "ready", to: "ca_validate", label: "answer_challenge: ready" },
    { from: "ca_validate", to: "http_get", label: "GET token path over HTTP :80" },
    { from: "http_get", to: "ca_validate", label: "keyAuthorization body" },
    { from: "ca_validate", to: "valid", label: "response matches" },
    { from: "ca_validate", to: "invalid", label: "wrong response or unreachable" },
    { from: "valid", to: "poll_auth", label: "status=valid" },
    { from: "invalid", to: "poll_auth", label: "status=invalid + problem" },
    { from: "poll_auth", to: "cleanup", label: "finally, even on error" },
    { from: "poll_auth", to: "finalize", label: "all authzs valid" },
    { from: "cleanup", to: "bind", label: "stop server once no challenges remain" },
    { from: "finalize", to: "issued", label: "finalize CSR; CA issues" },
    { from: "issued", to: "poll_issue", label: "order response / certificate URL" },
    { from: "poll_issue", to: "split", label: "fullchain PEM" },
    { from: "split", to: "lineage", label: "cert + chain + private key" },
    { from: "lineage", to: "live", label: "new renewable lineage" },
    { from: "live", to: "result", label: "certificate paths available" },
  ],
  style: { box, circle },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
