// OpenSSH public-key login permission checks, based on https://github.com/openssh/openssh-portable/blob/master/auth2.c,
// https://github.com/openssh/openssh-portable/blob/master/auth2-pubkey.c,
// https://github.com/openssh/openssh-portable/blob/master/auth2-pubkeyfile.c,
// https://github.com/openssh/openssh-portable/blob/master/auth.c, and
// https://man.openbsd.org/sshd_config.5. Follows one user's public-key attempt through account policy,
// authorized-key lookup and per-key restrictions, proof of private-key possession, and AuthenticationMethods.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "request", class: "box", x: 20, y: 210, label: "SSH_MSG_USERAUTH_REQUEST\nuser + publickey + key", fill: "#fef3c7", width: 190, height: 70 },
    { id: "account", class: "box", x: 265, y: 205, label: "auth.c: allowed_user()\nvalid account, not locked;\nAllowUsers / AllowGroups, etc.", fill: "#dbeafe", width: 235, height: 88 },
    { id: "method", class: "box", x: 550, y: 200, label: "auth2.c: input_userauth_request()\nmethod enabled + MaxAuthTries\ndispatch publickey", fill: "#dbeafe", width: 245, height: 88 },
    { id: "pubkey", class: "box", x: 845, y: 200, label: "auth2-pubkey.c: userauth_pubkey()\noffer probe, then signed request", fill: "#dbeafe", width: 245, height: 76 },
    { id: "keys", class: "box", x: 1140, y: 55, label: "AuthorizedKeysCommand\nthen AuthorizedKeysFile\nconfigured key sources", fill: "#e0e7ff", width: 230, height: 78 },
    { id: "keyopts", class: "box", x: 1430, y: 75, label: "auth2-pubkeyfile.c\nauth_check_authkeys_file()\nfrom=, expiry-time, cert principals", fill: "#dbeafe", width: 250, height: 88 },
    { id: "signature", class: "box", x: 1145, y: 335, label: "sshkey_verify()\nverify signature over session-bound\nuserauth request", fill: "#dbeafe", width: 245, height: 78 },
    { id: "complete", class: "box", x: 1740, y: 210, label: "auth2.c: authentication\nAuthenticationMethods satisfied?\nAuthenticated or continue methods", fill: "#dcfce7", width: 250, height: 88 },
    { id: "denied", class: "box", x: 550, y: 410, label: "Rejected attempt\naccount / key / signature policy\nfailed; client may try another method", fill: "#fee2e2", width: 260, height: 78 },
  ] satisfies Node[],
  edges: [
    { from: "request", to: "account" },
    { from: "account", to: "denied", label: "account disallowed" },
    { from: "account", to: "method", label: "account eligible" },
    { from: "method", to: "denied", label: "disabled / attempts exceeded" },
    { from: "method", to: "pubkey" },
    { from: "pubkey", to: "keys", label: "key offered; check authorization" },
    { from: "keys", to: "keyopts", label: "matching authorized key line" },
    { from: "keyopts", to: "pubkey", label: "probe accepted: request signature" },
    { from: "pubkey", to: "signature", label: "signed request" },
    { from: "signature", to: "denied", label: "invalid signature" },
    { from: "signature", to: "complete", label: "valid signature + key restrictions pass" },
    { from: "keyopts", to: "denied", label: "key or from= restriction fails" },
    { from: "complete", to: "denied", label: "more configured methods required" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
