// FastAPI request/response path, based on https://fastapi.tiangolo.com/tutorial/first-steps/,
// https://fastapi.tiangolo.com/tutorial/dependencies/, https://fastapi.tiangolo.com/tutorial/response-model/,
// https://fastapi.tiangolo.com/tutorial/handling-errors/, and the Starlette application stack at
// https://github.com/Kludex/starlette/blob/main/starlette/applications.py.
// It follows an HTTP request through ASGI middleware, route matching, dependency resolution and
// endpoint execution, then shows response-model filtering and both normal and exception replies.

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
    { id: "client", class: "box", x: 20, y: 180, label: "HTTP client\nmethod, path, headers, body" },
    { id: "server", class: "box", x: 270, y: 180, label: "ASGI server\nscope, receive, send" },
    { id: "fastapi-app", class: "box", x: 520, y: 180, label: "FastAPI / Starlette app\n__call__ → middleware_stack" },
    { id: "server-error", class: "box", x: 800, y: 40, label: "ServerErrorMiddleware\nuncaught errors → 500" },
    { id: "exception-middleware", class: "box", x: 800, y: 330, label: "ExceptionMiddleware\nHTTPException + validation handlers" },
    { id: "router", class: "box", x: 1080, y: 180, label: "Starlette Router\nmatch path + method" },
    { id: "api-route", class: "box", x: 1360, y: 180, label: "FastAPI APIRoute\nrequest handler" },
    { id: "request-deps", class: "box", x: 1640, y: 40, label: "Read body; solve_dependencies\npath/query/header/cookie + Depends\nPydantic validation" },
    { id: "endpoint", class: "box", x: 1640, y: 330, label: "Path operation function\nvalidated arguments + injected values" },
    { id: "response-model", class: "box", x: 1360, y: 490, label: "serialize_response\nresponse_model validate + filter" },
    { id: "response", class: "box", x: 800, y: 560, label: "Starlette Response\nJSONResponse / status + headers + body" },
    { id: "send", class: "box", x: 270, y: 560, label: "ASGI send\nhttp.response.start + body" },
  ],
  edges: [
    { from: "client", to: "server", label: "HTTP request bytes" },
    { from: "server", to: "fastapi-app", label: "ASGI scope + receive/send" },
    { from: "fastapi-app", to: "server-error", label: "enter outer middleware" },
    { from: "server-error", to: "exception-middleware", label: "middleware stack; exceptions pass outward" },
    { from: "exception-middleware", to: "router", label: "dispatch ASGI request" },
    { from: "router", to: "api-route", label: "matched APIRoute" },
    { from: "api-route", to: "request-deps", label: "read request body; prepare dependant" },
    { from: "request-deps", to: "endpoint", label: "validated values + dependency results" },
    { from: "endpoint", to: "response-model", label: "return value" },
    { from: "response-model", to: "response", label: "filtered data → response class" },
    { from: "response", to: "exception-middleware", label: "response unwinds middleware" },
    { from: "exception-middleware", to: "server-error", label: "response unwinds outer stack" },
    { from: "server-error", to: "send", label: "ASGI response messages" },
    { from: "send", to: "client", label: "HTTP status, headers, body" },
    { from: "request-deps", to: "exception-middleware", label: "RequestValidationError → 422 handler" },
    { from: "endpoint", to: "exception-middleware", label: "HTTPException → status/detail handler" },
    { from: "endpoint", to: "server-error", label: "unhandled exception → 500" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
