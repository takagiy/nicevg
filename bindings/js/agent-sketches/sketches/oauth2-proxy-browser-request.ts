// OAuth2 Proxy request handling, based on https://github.com/oauth2-proxy/oauth2-proxy/blob/master/oauthproxy.go and https://github.com/oauth2-proxy/oauth2-proxy/blob/master/pkg/sessions/cookie/session_store.go.
// This follows an unauthenticated browser request through provider login and callback, then follows its authenticated retry to the upstream response.

import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
  x: number;
  y: number;
  parent?: string;
  label: string;
  fill?: string;
  width?: number;
  height?: number;
};

const nodes: Node[] = [
  { id: "browser", class: "circle", x: 30, y: 190, label: "Browser", fill: "#fef3c7" },
  { id: "listener", class: "box", x: 220, y: 80, label: "proxyhttp.Server\nHTTP request listener", parent: "proxy", fill: "#dbeafe" },
  { id: "preauth", class: "box", x: 220, y: 185, label: "preAuthChain\nrequest scope / client IP", parent: "proxy", fill: "#dbeafe" },
  { id: "mux", class: "box", x: 220, y: 300, label: "serveMux\nroute /oauth2/* or app path", parent: "proxy", fill: "#dbeafe" },
  { id: "session", class: "box", x: 420, y: 300, label: "sessionChain\nload cookie session", parent: "proxy", fill: "#dbeafe" },
  { id: "store", class: "box", x: 420, y: 405, label: "SessionStore\nverify / decode cookie", parent: "proxy", fill: "#e0e7ff" },
  { id: "dispatch", class: "box", x: 630, y: 300, label: "OAuthProxy.Proxy\ngetAuthenticatedSession", parent: "proxy", fill: "#dbeafe" },
  { id: "start", class: "box", x: 630, y: 80, label: "OAuthStart\nredirect to provider", parent: "proxy", fill: "#dbeafe" },
  { id: "idp", class: "box", x: 850, y: 80, label: "OAuth / OIDC provider\nauthorize then redirect", fill: "#fce7f3" },
  { id: "callback", class: "box", x: 850, y: 190, label: "OAuthCallback\nvalidate state, exchange code", parent: "proxy", fill: "#dbeafe" },
  { id: "save", class: "box", x: 850, y: 300, label: "SaveSession\nSet-Cookie session", parent: "proxy", fill: "#dbeafe" },
  { id: "headers", class: "box", x: 850, y: 405, label: "headersChain\nX-Forwarded-User / Email", parent: "proxy", fill: "#dbeafe" },
  { id: "upstream", class: "box", x: 1060, y: 405, label: "upstreamProxy\nconfigured upstream handler", parent: "proxy", fill: "#dbeafe" },
  { id: "app", class: "box", x: 1280, y: 405, label: "Protected application", fill: "#dcfce7" },
  { id: "response", class: "circle", x: 1280, y: 190, label: "HTTP response", fill: "#fef3c7" },
];

const edges = [
  { from: "browser", to: "listener", label: "GET /app + Cookie?" },
  { from: "listener", to: "preauth", label: "http.Request" },
  { from: "preauth", to: "mux", label: "scoped request" },
  { from: "mux", to: "session", label: "application route" },
  { from: "session", to: "store", label: "_oauth2_proxy cookie" },
  { from: "store", to: "session", label: "SessionState or no session" },
  { from: "session", to: "dispatch", label: "request + session scope" },
  { from: "dispatch", to: "start", label: "ErrNeedsLogin" },
  { from: "start", to: "browser", label: "302 Location /oauth2/start" },
  { from: "browser", to: "start", label: "GET /oauth2/start" },
  { from: "start", to: "idp", label: "302 authorization URL + state" },
  { from: "idp", to: "browser", label: "redirect with code + state" },
  { from: "browser", to: "callback", label: "GET /oauth2/callback" },
  { from: "callback", to: "idp", label: "code exchange / token" },
  { from: "idp", to: "callback", label: "tokens + provider identity" },
  { from: "callback", to: "save", label: "SessionState" },
  { from: "save", to: "browser", label: "Set-Cookie; redirect to original URL" },
  { from: "browser", to: "listener", label: "retry GET /app + cookie" },
  { from: "dispatch", to: "headers", label: "authorized SessionState" },
  { from: "headers", to: "upstream", label: "identity headers + request" },
  { from: "upstream", to: "app", label: "proxied HTTP request" },
  { from: "app", to: "upstream", label: "status + response body" },
  { from: "upstream", to: "browser", label: "proxied response" },
  { from: "dispatch", to: "browser", label: "403 if authorization fails" },
];

const sketch = {
  nodes: [
    { id: "proxy", class: "box", x: 185, y: 40, label: "OAuth2 Proxy process", width: 920, height: 500, fill: "#eff6ff" },
    ...nodes,
  ],
  edges,
  style: { box, circle },
} satisfies Sketch<Node>;

export default sketch;
