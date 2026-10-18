// cert-manager, based on https://cert-manager.io/docs/concepts/acme-orders-challenges/,
// https://cert-manager.io/docs/usage/certificate/, https://cert-manager.io/docs/configuration/acme/http01/,
// and https://cert-manager.io/docs/tutorials/acme/http-validation/. Shows an ACME CertificateRequest
// unfolding into an Order and HTTP-01 Challenge, the temporary solver route, and the signed
// certificate and private key returning to the Certificate's target Secret.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "request", class: "box", x: 30, y: 170, label: "CertificateRequest\nCSR + issuerRef\nprivate key generated/stored", fill: "#e0e7ff" },
    { id: "order", class: "box", x: 290, y: 170, label: "ACME Order\ncreated by ACME issuer\none certificate request", fill: "#fce7f3" },
    { id: "challenge", class: "box", x: 550, y: 170, label: "Challenge\nper DNS identifier\nHTTP-01 token + key authorization", fill: "#fce7f3" },
    { id: "acme", class: "box", x: 1050, y: 30, label: "ACME server\nrequests /.well-known/\nacme-challenge/<token>", fill: "#fef3c7" },
    { id: "ingress", class: "box", x: 1050, y: 310, label: "Temporary Ingress\npath routes challenge\nto solver Service", fill: "#dcfce7" },
    { id: "service", class: "box", x: 1300, y: 310, label: "Temporary Service\nfor acmesolver Pod", fill: "#dcfce7" },
    { id: "pod", class: "box", x: 1530, y: 310, label: "acmesolver Pod\nserves key authorization", fill: "#dcfce7" },
    { id: "secret", class: "box", x: 290, y: 420, label: "Kubernetes Secret\nCertificate.spec.secretName\ntls.crt + tls.key", fill: "#e0e7ff" },
  ] satisfies Node[],
  edges: [
    { from: "request", to: "order", label: "ACME issuer creates Order" },
    { from: "order", to: "challenge", label: "creates one Challenge per DNS name" },
    { from: "challenge", to: "ingress", label: "HTTP-01 solver presents token path" },
    { from: "ingress", to: "service", label: "/.well-known/acme-challenge/*" },
    { from: "service", to: "pod", label: "forwards request" },
    { from: "acme", to: "ingress", label: "public validation request" },
    { from: "pod", to: "acme", label: "key authorization response" },
    { from: "challenge", to: "order", label: "valid / failed status" },
    { from: "request", to: "secret", label: "store tls.crt + tls.key" },
    { from: "order", to: "request", label: "final signed certificate" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
