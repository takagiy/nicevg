// NYC311 service request lifecycle, based on https://portal.311.nyc.gov/about-nyc-311/,
// https://portal.311.nyc.gov/article/?kanumber=KA-01066, and
// https://www.nyc.gov/assets/home/html/311/311_mobile_faq.shtml.
// It follows a resident's non-emergency request from mobile submission through agency work,
// closure and status or satisfaction messages back to the resident.

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
    { id: "resident", class: "box", x: 30, y: 170, label: "Resident\nissue + location" },
    { id: "mobile-request", class: "box", x: 280, y: 170, label: "NYC311 Mobile\nRequest form" },
    { id: "nyc311", class: "box", x: 550, y: 170, label: "NYC311\nroute SR in real time" },
    { id: "agency", class: "box", x: 850, y: 170, label: "Responsible City agency\ninspection / repair / ticket" },
    { id: "sr", class: "box", x: 850, y: 360, label: "Service Request (SR)\nSR number · status · resolution" },
    { id: "notifications", class: "box", x: 550, y: 370, label: "Email or SMS\nconfirmation + status updates" },
    { id: "survey", class: "box", x: 280, y: 370, label: "Satisfaction survey\nafter agency closure" },
  ],
  edges: [
    { from: "resident", to: "mobile-request", label: "tap Request; report one issue at a location" },
    { from: "mobile-request", to: "nyc311", label: "submit request" },
    { from: "nyc311", to: "agency", label: "SR routed to appropriate agency" },
    { from: "nyc311", to: "sr", label: "create SR; return tracking number" },
    { from: "agency", to: "sr", label: "actions, then closed + resolution information" },
    { from: "sr", to: "notifications", label: "confirmation; each agency action" },
    { from: "notifications", to: "resident", label: "if email address or phone was provided" },
    { from: "agency", to: "survey", label: "after closure, NYC311 sends survey" },
    { from: "survey", to: "resident", label: "email or text for feedback" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
