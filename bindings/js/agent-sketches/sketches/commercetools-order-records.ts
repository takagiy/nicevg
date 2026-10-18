// commercetools order records, based on the official Carts and Orders overview and Orders API.
// Shows the durable Order snapshot, its item and customer references, and the payment, shipping,
// delivery, and return records associated with it.
// Sources: https://docs.commercetools.com/api/carts-orders-overview
//          https://docs.commercetools.com/api/projects/orders
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const sketch = {
  nodes: [
    { id: "order", class: "box", x: 420, y: 250, label: "Order\nPK id · version · orderNumber\ncustomerId · cart · orderState\nstate · totalPrice · taxedPrice\nbillingAddress · shippingAddress", width: 290, height: 116 },
    { id: "customer", class: "box", x: 40, y: 60, label: "Customer\nPK id · email · customerNumber\nOrders can also be guest orders\n(anonymousId instead)", width: 255, height: 90 },
    { id: "cart", class: "box", x: 40, y: 250, label: "Cart (source snapshot)\nPK id · version · customerId\nlineItems · totalPrice\nOrder keeps a cart reference", width: 255, height: 90 },
    { id: "line_item", class: "box", x: 40, y: 470, label: "LineItem (embedded)\nid · productId · variantId · sku\nquantity · price · totalPrice\nstate[] · discountedPricePerQuantity[]", width: 290, height: 105 },
    { id: "variant", class: "box", x: 400, y: 480, label: "LineItem.variant snapshot\nid · sku · key · prices[]\nattributes · images[]\nProduct changes do not refresh it", width: 290, height: 92 },
    { id: "state", class: "box", x: 760, y: 480, label: "State (reference)\nPK id · key · type\nLineItem.state[] carries quantity\nand the referenced State", width: 260, height: 88 },
    { id: "payment", class: "box", x: 800, y: 60, label: "Payment\nPK id · amountPlanned · paymentStatus\npaymentMethodInfo · transactions[]\nOrder.paymentInfo.payments[]", width: 300, height: 100 },
    { id: "transaction", class: "box", x: 1160, y: 60, label: "Transaction (embedded)\nid · timestamp · type · amount\nstate · interactionId\ninterfaceId · custom", width: 260, height: 90 },
    { id: "shipping_info", class: "box", x: 800, y: 250, label: "ShippingInfo (embedded)\nshippingMethod · shippingMethodName\nprice · shippingRate\ntaxRate · deliveries[]", width: 290, height: 100 },
    { id: "shipping_method", class: "box", x: 1160, y: 250, label: "ShippingMethod\nPK id · key · name\nzoneRates · taxCategory\nShippingInfo keeps selected method data", width: 275, height: 92 },
    { id: "delivery", class: "box", x: 800, y: 440, label: "Delivery (embedded)\nid · key · items[] · parcels[]\nitems identify delivered line items\nby lineItemId and quantity", width: 290, height: 90 },
    { id: "parcel", class: "box", x: 1160, y: 440, label: "Parcel (embedded)\nid · createdAt · measurements\ntrackingData: trackingId · carrier\ntrackingUrl", width: 260, height: 90 },
    { id: "return_item", class: "box", x: 420, y: 650, label: "ReturnItem (embedded)\nid · quantity · lineItemId\nshipmentState · paymentState\ncontained in Order.returnInfo.items[]", width: 290, height: 90 },
  ],
  edges: [
    { from: "order", to: "customer", label: "customerId → Customer.id (when set)" },
    { from: "order", to: "cart", label: "cart.id reference" },
    { from: "order", to: "line_item", label: "lineItems[]; copied with price and fields" },
    { from: "line_item", to: "variant", label: "variant snapshot; productId / variantId" },
    { from: "line_item", to: "state", label: "state[].state → State.id" },
    { from: "order", to: "payment", label: "paymentInfo.payments[] → Payment.id" },
    { from: "payment", to: "transaction", label: "transactions[]" },
    { from: "order", to: "shipping_info", label: "shippingInfo (Single mode)" },
    { from: "shipping_info", to: "shipping_method", label: "shippingMethod reference" },
    { from: "shipping_info", to: "delivery", label: "deliveries[]" },
    { from: "delivery", to: "parcel", label: "parcels[]" },
    { from: "order", to: "return_item", label: "returnInfo[].items[]; lineItemId links item" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string; width: number; height: number }>;

export default sketch;
