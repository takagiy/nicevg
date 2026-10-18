// Matrix Synapse room-message send path.
// Sources: https://github.com/matrix-org/synapse/blob/develop/synapse/rest/client/room.py,
// https://github.com/matrix-org/synapse/blob/develop/synapse/handlers/message.py,
// https://github.com/matrix-org/synapse/blob/develop/docs/workers.md,
// https://spec.matrix.org/latest/client-server-api/#put_matrixclientv3roomsroomidsendeventtypetxnid
// Traces an authenticated client's PUT through transaction handling, event creation and persistence
// to the event_id response, including validation rejection and the configured worker handoff.

import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
  x: number;
  y: number;
  parent?: string;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const nodes: Node[] = [
  { id: "client", class: "circle", x: 20, y: 180, label: "Matrix client\nPUT message" },
  { id: "proxy", class: "box", x: 190, y: 180, label: "Reverse proxy\nroute /send" },
  { id: "listener", class: "box", x: 370, y: 180, label: "Synapse client\nHTTP listener" },
  { id: "servlet", class: "box", x: 550, y: 180, label: "RoomSendEvent\nRestServlet" },
  { id: "auth", class: "box", x: 730, y: 80, label: "Auth.get_user_by_req\naccess token → Requester" },
  { id: "unauth", class: "box", x: 730, y: 20, label: "401 / credential\nerror response" },
  { id: "put", class: "box", x: 730, y: 250, label: "on_PUT\nset txn_id tag" },
  { id: "txn", class: "box", x: 920, y: 250, label: "TransactionQueue\nfetch_or_execute_request" },
  { id: "cached", class: "box", x: 1100, y: 330, label: "Cached transaction\nresponse" },
  { id: "do", class: "box", x: 1100, y: 190, label: "_do(requester, room_id,\nevent_type, txn_id)" },
  { id: "json", class: "box", x: 1280, y: 190, label: "Parse JSON object\nrequest content" },
  { id: "dict", class: "box", x: 1460, y: 190, label: "event_dict\ntype, content, room_id, sender" },
  { id: "handler", class: "box", x: 1660, y: 190, label: "EventCreationHandler\ncreate_and_send_nonmember_event" },
  { id: "ban", class: "box", x: 1870, y: 20, label: "Shadow-ban delay\nthen synthetic event_id" },
  { id: "rate", class: "box", x: 1870, y: 190, label: "Request / room\nrate limits" },
  { id: "roomver", class: "box", x: 2070, y: 190, label: "Room version +\nevent builder" },
  { id: "prev", class: "box", x: 2270, y: 190, label: "Read forward extremities\nprev_events + room state" },
  { id: "event", class: "box", x: 2470, y: 190, label: "Create event\nIDs, depth, auth refs" },
  { id: "validate", class: "box", x: 2670, y: 190, label: "Validate event\nshape + room version" },
  { id: "reject", class: "box", x: 2670, y: 20, label: "SynapseError\n4xx / 5xx response" },
  { id: "authrules", class: "box", x: 2870, y: 190, label: "Check room auth rules\nagainst event context" },
  { id: "rejectauth", class: "box", x: 2870, y: 20, label: "AuthError 403\nforbidden event" },
  { id: "creator", class: "box", x: 3070, y: 190, label: "Event creator\nhandle_new_client_event" },
  { id: "persist", class: "box", x: 3270, y: 190, label: "Event persister\nHTTP replication (if configured)" },
  { id: "db", class: "box", x: 3470, y: 190, label: "Database\nevents + event_json + relations" },
  { id: "stream", class: "box", x: 3670, y: 190, label: "events stream\nstream ordering" },
  { id: "notify", class: "box", x: 3870, y: 190, label: "Notifier +\npost-persist callbacks" },
  { id: "federation", class: "box", x: 4070, y: 100, label: "Federation sender\nremote joined servers" },
  { id: "sync", class: "box", x: 4070, y: 280, label: "Local clients\nobserve via /sync" },
  { id: "reply", class: "box", x: 4270, y: 190, label: "RoomSendEventRestServlet\nreturn { event_id }" },
  { id: "response", class: "circle", x: 4480, y: 190, label: "HTTP 200\nclient receives event_id" },
];

const edges = [
  { from: "client", to: "proxy", label: "PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message/{txnId}" },
  { from: "proxy", to: "listener", label: "HTTP request" },
  { from: "listener", to: "servlet", label: "route /rooms/{room_id}/send/{event_type}" },
  { from: "servlet", to: "auth", label: "authenticate bearer token" },
  { from: "auth", to: "unauth", label: "invalid / missing token" },
  { from: "auth", to: "put", label: "Requester (user, device)" },
  { from: "put", to: "txn", label: "request + txn_id" },
  { from: "txn", to: "cached", label: "same transaction already handled" },
  { from: "cached", to: "reply", label: "reuse prior response" },
  { from: "txn", to: "do", label: "first execution" },
  { from: "do", to: "json", label: "body bytes" },
  { from: "json", to: "dict", label: "content object" },
  { from: "dict", to: "handler", label: "event_dict + requester + txn_id" },
  { from: "handler", to: "ban", label: "shadow-banned requester" },
  { from: "ban", to: "reply", label: "fake event_id; conceal ban" },
  { from: "handler", to: "rate", label: "normal send" },
  { from: "rate", to: "roomver", label: "allowed" },
  { from: "rate", to: "reject", label: "limit exceeded" },
  { from: "roomver", to: "prev", label: "version-specific builder" },
  { from: "prev", to: "event", label: "latest room DAG + auth state" },
  { from: "event", to: "validate", label: "unpersisted event + context" },
  { from: "validate", to: "reject", label: "invalid event" },
  { from: "validate", to: "authrules", label: "valid event" },
  { from: "authrules", to: "rejectauth", label: "sender lacks permission" },
  { from: "authrules", to: "creator", label: "authorized event" },
  { from: "creator", to: "persist", label: "event + EventContext" },
  { from: "persist", to: "db", label: "persist event DAG data" },
  { from: "db", to: "stream", label: "committed stream position" },
  { from: "stream", to: "notify", label: "new event position" },
  { from: "notify", to: "federation", label: "enqueue/send room event" },
  { from: "notify", to: "sync", label: "wake sync long-polls" },
  { from: "notify", to: "reply", label: "persisted event.event_id" },
  { from: "reply", to: "response", label: "200 { event_id }" },
  { from: "reject", to: "response", label: "error JSON + status" },
  { from: "rejectauth", to: "response", label: "403 error JSON" },
];

export default {
  nodes,
  edges,
  style: { box, circle },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
