// Django request/response path, based on https://github.com/django/django/blob/main/django/core/handlers/wsgi.py,
// https://github.com/django/django/blob/main/django/core/handlers/base.py, and https://docs.djangoproject.com/en/stable/topics/http/urls/.
// It follows a WSGI request into Django's handler, through URL resolution and the view,
// then back as an HttpResponse, including exception handling and deferred rendering.

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
    { id: "wsgi", class: "box", x: 30, y: 140, label: "WSGI server\n(environ, start_response)" },
    { id: "wsgi-handler", class: "box", x: 280, y: 140, label: "WSGIHandler.__call__\nconstruct WSGIRequest" },
    { id: "middleware-chain", class: "box", x: 560, y: 140, label: "BaseHandler.get_response\nrequest middleware chain" },
    { id: "resolver", class: "box", x: 850, y: 35, label: "resolve_request\nROOT_URLCONF → URL resolver\npath_info → ResolverMatch" },
    { id: "view", class: "box", x: 1130, y: 140, label: "process_view hooks\nthen resolved view callback\n(request, args, kwargs)" },
    { id: "exception-template", class: "box", x: 850, y: 310, label: "Exception middleware\n+ template-response middleware\nrender deferred response" },
    { id: "http-response", class: "box", x: 560, y: 390, label: "HttpResponse\nstatus, headers, cookies, body" },
  ],
  edges: [
    { from: "wsgi", to: "wsgi-handler", label: "HTTP environ" },
    { from: "wsgi-handler", to: "middleware-chain", label: "WSGIRequest" },
    { from: "middleware-chain", to: "resolver", label: "request.path_info" },
    { from: "resolver", to: "view", label: "callback + URL args/kwargs" },
    { from: "middleware-chain", to: "view", label: "process_view(request, view)" },
    { from: "view", to: "exception-template", label: "exception → process_exception\nTemplateResponse → hooks + render()" },
    { from: "exception-template", to: "middleware-chain", label: "HttpResponse" },
    { from: "view", to: "middleware-chain", label: "HttpResponse (or response short-circuit)" },
    { from: "middleware-chain", to: "http-response", label: "response middleware unwinds" },
    { from: "http-response", to: "wsgi-handler", label: "response" },
    { from: "wsgi-handler", to: "wsgi", label: "start_response(status, headers)\niterate response body" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
