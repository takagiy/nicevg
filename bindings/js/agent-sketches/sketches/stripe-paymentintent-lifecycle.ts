// Stripe PaymentIntents, based on https://docs.stripe.com/api/payment_intents
// and https://docs.stripe.com/api/payment_intents/confirm. Shows a PaymentIntent's
// status transitions, including authentication, failed attempts, manual capture and cancellation.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes = [
  { id: "create", class: "box", x: 30, y: 150, label: "Create PaymentIntent\namount, currency\ncapture_method, confirmation_method" },
  { id: "needs_method", class: "box", x: 320, y: 150, label: "requires_payment_method\nno usable payment method yet" },
  { id: "needs_confirmation", class: "box", x: 620, y: 150, label: "requires_confirmation\nready for confirmation" },
  { id: "confirm", class: "box", x: 920, y: 150, label: "Confirm\nPOST /payment_intents/:id/confirm\nattempt payment" },
  { id: "action", class: "box", x: 1220, y: 30, label: "requires_action\nnext_action tells client\nwhat customer must do" },
  { id: "processing", class: "box", x: 1220, y: 260, label: "processing\npayment method is processing\nresult not final yet" },
  { id: "capture", class: "box", x: 1520, y: 150, label: "requires_capture\nauthorized; amount_capturable\nserver must capture" },
  { id: "capture_call", class: "box", x: 1820, y: 150, label: "Capture\nPOST /payment_intents/:id/capture" },
  { id: "succeeded", class: "box", x: 2120, y: 150, label: "succeeded\npayment completed" },
  { id: "failed", class: "box", y: 440, x: 920, label: "Attempt fails\nPaymentIntent returns to retry path" },
  { id: "canceled", class: "box", x: 1520, y: 500, label: "canceled\nterminal; no further charges\nremaining capturable amount refunded" },
  { id: "cancel_call", class: "box", x: 920, y: 640, label: "Cancel\nPOST /payment_intents/:id/cancel\nonly in cancelable statuses" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "create", to: "needs_method", label: "created status" },
    { from: "needs_method", to: "needs_confirmation", label: "attach payment_method" },
    { from: "needs_method", to: "confirm", label: "confirm with method" },
    { from: "needs_confirmation", to: "confirm", label: "automatic: client SDK or server" },
    { from: "confirm", to: "action", label: "customer authentication needed" },
    { from: "action", to: "needs_confirmation", label: "manual: action complete; server reconfirms" },
    { from: "action", to: "processing", label: "automatic: action handled; continues" },
    { from: "confirm", to: "processing", label: "asynchronous payment method" },
    { from: "confirm", to: "succeeded", label: "payment succeeds; automatic capture" },
    { from: "confirm", to: "capture", label: "payment succeeds; capture_method=manual" },
    { from: "processing", to: "succeeded", label: "payment completes" },
    { from: "processing", to: "needs_method", label: "payment fails" },
    { from: "confirm", to: "failed", label: "payment attempt fails" },
    { from: "failed", to: "needs_method", label: "new payment method" },
    { from: "failed", to: "canceled", label: "confirmation limit reached" },
    { from: "capture", to: "capture_call", label: "server capture request" },
    { from: "capture_call", to: "succeeded", label: "captured" },
    { from: "needs_method", to: "cancel_call", label: "cancel allowed" },
    { from: "needs_confirmation", to: "cancel_call", label: "cancel allowed" },
    { from: "action", to: "cancel_call", label: "cancel allowed" },
    { from: "capture", to: "cancel_call", label: "cancel; remaining amount refunded" },
    { from: "processing", to: "cancel_call", label: "rarely cancelable" },
    { from: "cancel_call", to: "canceled", label: "cancel succeeds" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
