// Traefik HTTP retry middleware, based on https://github.com/traefik/traefik/blob/master/pkg/middlewares/retry/retry.go
// and https://doc.traefik.io/traefik/reference/routing-configuration/http/middlewares/retry/.
// It shows how Retry.ServeHTTP decides whether to resend an eligible, replayable request after a
// configured status response, holds that response back, and eventually returns the last response.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "serve", class: "box", x: 40, y: 190, label: "retry.ServeHTTP\nmethod eligible? attempts left?", width: 230, height: 76, fill: "#dbeafe" },
    { id: "reusable", class: "box", x: 345, y: 65, label: "mirror.NewReusableRequest\nclone body for another attempt", width: 230, height: 76, fill: "#fef3c7" },
    { id: "next", class: "box", x: 660, y: 190, label: "r.next.ServeHTTP\nservice handler / upstream", width: 225, height: 76, fill: "#dcfce7" },
    { id: "writer", class: "box", x: 965, y: 190, label: "responseWriter.WriteHeader\nstatusCodeRange.Contains(code)?", width: 250, height: 76, fill: "#ede9fe" },
    { id: "backoff", class: "box", x: 665, y: 365, label: "backoff.RetryNotify\nnewBackOff + attempt counter", width: 225, height: 76, fill: "#ffedd5" },
    { id: "listener", class: "box", x: 365, y: 385, label: "listener.Retried(req, attempts)\nretry observation", width: 220, height: 66, fill: "#f1f5f9" },
    { id: "client", class: "box", x: 1295, y: 190, label: "rw: original ResponseWriter\nfinal headers + status + body", width: 240, height: 76, fill: "#fee2e2" },
  ] satisfies Node[],
  edges: [
    { from: "serve", to: "reusable", label: "HTTP status retries enabled" },
    { from: "reusable", to: "next", label: "reusable request clone per try" },
    { from: "serve", to: "next", label: "status retry disabled: original request" },
    { from: "next", to: "writer", label: "upstream response headers" },
    { from: "writer", to: "backoff", label: "configured status; withhold response" },
    { from: "backoff", to: "listener", label: "before next attempt" },
    { from: "listener", to: "serve", label: "retry; incremented attempt" },
    { from: "writer", to: "client", label: "not configured, timed out, or final try" },
    { from: "serve", to: "client", label: "attempts exhausted: call next with rw" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
