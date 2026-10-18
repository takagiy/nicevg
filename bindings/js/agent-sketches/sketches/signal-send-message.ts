// Signal message delivery, based on the Sesame session-management specification
// (https://signal.org/docs/specifications/sesame/) and the Signal service's description of
// queued delivery (https://support.signal.org/hc/en-us/articles/5532268300186-Disappearing-Messages-with-a-Linked-Device).
// Follows a user sending one message: the client encrypts separately for recipient devices,
// the service routes ciphertext into per-device mailboxes, and each device fetches and decrypts it.

import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "sender", class: "circle", x: 30, y: 180, label: "Sender's Signal client\ncompose and send" },
    { id: "records", class: "box", x: 280, y: 70, label: "Sesame UserRecord / DeviceRecords\nactive sessions; device list" },
    { id: "crypto", class: "box", x: 570, y: 170, label: "Signal Protocol sessions\nencrypt plaintext per device" },
    { id: "service", class: "box", x: 880, y: 170, label: "Signal service\nverify current recipient devices" },
    { id: "mailboxes", class: "box", x: 1190, y: 280, label: "Per-device mailboxes\nqueued ciphertext" },
    { id: "recipient", class: "circle", x: 1510, y: 280, label: "Recipient's Signal device\nfetch, decrypt, display" },
    { id: "receipt", class: "box", x: 1190, y: 30, label: "Sender's Signal client\nupdate delivery status" },
  ],
  edges: [
    { from: "sender", to: "records", label: "recipient UserID; local session state" },
    { from: "records", to: "crypto", label: "non-stale active sessions + DeviceIDs" },
    { from: "crypto", to: "service", label: "recipient UserID + per-device ciphertexts and DeviceIDs" },
    { from: "service", to: "crypto", label: "device list stale / recipient not currently in use: refresh and retry" },
    { from: "service", to: "mailboxes", label: "accepted encrypted messages + sender UserID and DeviceID" },
    { from: "mailboxes", to: "recipient", label: "fetch ciphertext + sender identifiers; remove fetched message" },
    { from: "recipient", to: "receipt", label: "optional delivery receipt for MessageID" },
  ],
  style: { box, circle },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
