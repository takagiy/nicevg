// Shopify Hydrogen's legacy cart action and the Storefront API cartLinesAdd mutation.
// Sources: https://shopify.dev/docs/storefronts/headless/legacy/hydrogen/cart/manage
// https://shopify.dev/docs/api/storefront/latest/mutations/cartlinesadd
// Shows an add-lines form through Hydrogen's route action and cart handler to Shopify,
// including cart ID cookie updates and the result returned to the caller.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

export default {
  nodes: [
    { id: "buyer", class: "box", x: 20, y: 170, label: "Buyer\nclicks Add to cart" },
    { id: "cart-form", class: "box", x: 220, y: 170, label: "CartForm\nroute=/cart\naction=LinesAdd" },
    { id: "post", class: "box", x: 440, y: 170, label: "POST /cart\nformData" },
    { id: "route-action", class: "box", x: 660, y: 170, label: "/cart action\nrequest + context" },
    { id: "form-input", class: "box", x: 880, y: 170, label: "CartForm.getFormInput\n{ action, inputs.lines }" },
    { id: "cart-handler", class: "box", x: 1100, y: 170, label: "context.cart\naddLines(lines)" },
    { id: "cart-id", class: "box", x: 1100, y: 350, label: "getCartId\nrequest cookie → cart ID" },
    { id: "storefront", class: "box", x: 1320, y: 170, label: "Storefront client\nGraphQL mutation" },
    { id: "shopify-api", class: "box", x: 1540, y: 170, label: "Storefront API\ncartLinesAdd(cartId, lines)" },
    { id: "payload", class: "box", x: 1540, y: 350, label: "CartLinesAddPayload\ncart + userErrors + warnings" },
    { id: "set-cart-id", class: "box", x: 1320, y: 520, label: "cart.setCartId\nresult.cart.id → Set-Cookie" },
    { id: "json-response", class: "box", x: 880, y: 520, label: "json(result, {status: 200, headers})\nroute response" },
    { id: "client-result", class: "box", x: 440, y: 520, label: "Caller receives\ncart result / userErrors" },
  ],
  edges: [
    { from: "buyer", to: "cart-form", label: "submit merchandiseId, quantity" },
    { from: "cart-form", to: "post", label: "HTML form submission" },
    { from: "post", to: "route-action", label: "request.formData()" },
    { from: "route-action", to: "form-input", label: "decode action and inputs" },
    { from: "form-input", to: "cart-handler", label: "LinesAdd → addLines" },
    { from: "cart-id", to: "cart-handler", label: "existing cart ID or none" },
    { from: "cart-handler", to: "storefront", label: "cart mutation" },
    { from: "storefront", to: "shopify-api", label: "GraphQL request" },
    { from: "shopify-api", to: "payload", label: "updated Cart; errors and warnings" },
    { from: "payload", to: "set-cart-id", label: "result.cart.id" },
    { from: "set-cart-id", to: "json-response", label: "Set-Cookie response header" },
    { from: "payload", to: "json-response", label: "result body" },
    { from: "json-response", to: "client-result", label: "HTTP 200 JSON" },
  ],
  style: { box },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string }>;

