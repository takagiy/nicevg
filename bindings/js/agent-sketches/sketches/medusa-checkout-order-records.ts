// Medusa checkout and order records, based on the official architecture and commerce module docs.
// Shows the cart snapshot becoming an order, its cross-module payment, product, inventory, and fulfillment links,
// and the external payment and shipping providers used at those boundaries.
// Sources: https://docs.medusajs.com/learn/introduction/architecture
//          https://docs.medusajs.com/resources/commerce-modules/cart/links-to-other-modules
//          https://docs.medusajs.com/resources/commerce-modules/order/links-to-other-modules
//          https://docs.medusajs.com/resources/commerce-modules/payment/payment-checkout-flow
//          https://docs.medusajs.com/resources/commerce-modules/payment/payment-collection
//          https://docs.medusajs.com/resources/commerce-modules/payment/payment-session
//          https://docs.medusajs.com/resources/commerce-modules/fulfillment/links-to-other-modules
//          https://docs.medusajs.com/resources/commerce-modules/inventory/inventory-in-flows
//          https://docs.medusajs.com/resources/commerce-modules/inventory/concepts
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const sketch = {
  nodes: [
    { id: "storefront", class: "box", x: 30, y: 280, label: "Storefront\nStore API: cart + complete\nreads payment session data", width: 230, height: 82 },
    { id: "workflow", class: "box", x: 330, y: 270, label: "completeCartWorkflow\nvalidates + authorizes payment\ncreates order + reservations", width: 250, height: 82 },
    { id: "cart", class: "box", x: 640, y: 30, label: "Cart\nPK id · currency_code · region_id\ncustomer_id · email · sales_channel_id\nsubtotal · tax_total · total\ncompleted_at", width: 280, height: 108 },
    { id: "address", class: "box", x: 30, y: 30, label: "Address\nPK id · customer_id\nfirst_name · last_name · address_1\ncity · country_code · postal_code", width: 255, height: 92 },
    { id: "line", class: "box", x: 640, y: 190, label: "LineItem\nPK id · cart_id · product_id · variant_id\ntitle · quantity · unit_price\nsubtotal · tax_total · total", width: 280, height: 98 },
    { id: "product", class: "box", x: 990, y: 30, label: "Product\nPK id · title · handle · status\nLineItem.product_id resolves here", width: 255, height: 76 },
    { id: "variant", class: "box", x: 990, y: 170, label: "ProductVariant\nPK id · product_id · sku · title\nmanage_inventory · inventory_items[]", width: 255, height: 82 },
    { id: "shipping", class: "box", x: 640, y: 360, label: "ShippingMethod (cart)\nPK id · cart_id · shipping_option_id\namount · data · metadata", width: 280, height: 82 },
    { id: "order", class: "box", x: 640, y: 540, label: "Order\nPK id · display_id · version · status\ncurrency_code · region_id · customer_id\nsubtotal · tax_total · total · metadata", width: 280, height: 100 },
    { id: "order_item", class: "box", x: 990, y: 350, label: "OrderLineItem\nPK id · title · product_id · variant_id\nquantity · unit_price · compare_at_unit_price\noriginal purchased item details", width: 300, height: 100 },
    { id: "order_item_version", class: "box", x: 990, y: 510, label: "OrderItem\nPK id · version · quantity · unit_price\ncompare_at_unit_price · metadata\nversioned item values", width: 290, height: 90 },
    { id: "order_ship", class: "box", x: 640, y: 700, label: "OrderShippingMethod\nPK id · order_id · shipping_option_id\nname · amount · data", width: 280, height: 82 },
    { id: "shipping_option", class: "box", x: 990, y: 700, label: "ShippingOption\nPK id · name · provider_id\nservice_zone_id · price_type\nsource for order shipping method", width: 280, height: 90 },
    { id: "payment_collection", class: "box", x: 30, y: 450, label: "PaymentCollection\nPK id · currency_code · amount\nauthorization / capture / refund state\ncart link, then order link", width: 280, height: 92 },
    { id: "payment_session", class: "box", x: 30, y: 600, label: "PaymentSession\nPK id · provider_id · amount · currency_code\nstatus · data · context\npending / requires_more / authorized / error", width: 290, height: 100 },
    { id: "provider", class: "box", x: 30, y: 770, label: "Payment provider\nMedusa provider module\ninitialize · authorize · capture · refund", width: 275, height: 80 },
    { id: "gateway", class: "box", x: 350, y: 770, label: "External payment system\nprovider session / intent ID in data\nmay request customer action (3DS)", width: 275, height: 82 },
    { id: "payment", class: "box", x: 350, y: 450, label: "Payment\nPK id · currency_code · amount\nauthorized_at · captured_at · data\ncreated when session is authorized", width: 270, height: 90 },
    { id: "inventory_item", class: "box", x: 1390, y: 20, label: "InventoryItem\nPK id · sku · requires_shipping\nunit_of_measure", width: 255, height: 78 },
    { id: "inventory_level", class: "box", x: 1390, y: 170, label: "InventoryLevel\nPK id · inventory_item_id · location_id\nstocked_quantity · reserved_quantity\nincoming_quantity", width: 280, height: 92 },
    { id: "reservation", class: "box", x: 1390, y: 340, label: "ReservationItem\nPK id · inventory_item_id · location_id\nline_item_id · quantity\ncreated when cart completes", width: 290, height: 92 },
    { id: "location", class: "box", x: 1390, y: 510, label: "StockLocation\nPK id · name · address_id\nselected by sales channel for reservation", width: 290, height: 82 },
    { id: "fulfillment", class: "box", x: 1390, y: 680, label: "Fulfillment\nPK id · location_id · provider_id\npacked_at · shipped_at · delivered_at\ndata · labels[]", width: 275, height: 88 },
    { id: "fulfillment_provider", class: "box", x: 1730, y: 690, label: "FulfillmentProvider\nPK id · is_enabled\nmanual or integration provider", width: 260, height: 78 },
    { id: "shipper", class: "box", x: 1730, y: 840, label: "External shipping system\nlabel + tracking response\nprovider-specific fulfillment data", width: 265, height: 82 },
    { id: "event_bus", class: "box", x: 990, y: 850, label: "Event Module\norder / payment / fulfillment events\npub-sub integration boundary", width: 275, height: 80 },
  ],
  edges: [
    { from: "storefront", to: "cart", label: "Store API cart fields" },
    { from: "storefront", to: "workflow", label: "POST complete cart" },
    { from: "workflow", to: "cart", label: "load cart + relationships" },
    { from: "cart", to: "address", label: "shipping_address / billing_address" },
    { from: "cart", to: "line", label: "line_items[]" },
    { from: "line", to: "product", label: "product_id (read-only link)" },
    { from: "line", to: "variant", label: "variant_id (read-only link)" },
    { from: "cart", to: "shipping", label: "shipping_methods[]" },
    { from: "shipping", to: "shipping_option", label: "shipping_option_id" },
    { from: "cart", to: "payment_collection", label: "stored one-to-one module link" },
    { from: "payment_collection", to: "payment_session", label: "payment_sessions[]" },
    { from: "payment_session", to: "provider", label: "provider_id + amount" },
    { from: "provider", to: "gateway", label: "initialize / authorize request" },
    { from: "gateway", to: "provider", label: "provider response / action required" },
    { from: "provider", to: "payment_session", label: "status + provider data" },
    { from: "payment_session", to: "storefront", label: "client token / requires_more" },
    { from: "workflow", to: "provider", label: "authorize during completion" },
    { from: "payment_session", to: "payment", label: "authorized session creates payment" },
    { from: "cart", to: "order", label: "cart_id; stored one-to-one link" },
    { from: "workflow", to: "order", label: "create order snapshot" },
    { from: "line", to: "order_item", label: "copy original item details" },
    { from: "order_item", to: "order_item_version", label: "versioned quantity / price" },
    { from: "order", to: "order_ship", label: "shipping_methods[]" },
    { from: "order_ship", to: "shipping_option", label: "shipping_option_id read-only link" },
    { from: "order", to: "payment_collection", label: "stored one-to-many link" },
    { from: "workflow", to: "reservation", label: "reserve ordered quantities" },
    { from: "variant", to: "inventory_item", label: "inventory_items[]" },
    { from: "inventory_item", to: "inventory_level", label: "levels by location" },
    { from: "inventory_level", to: "location", label: "location_id" },
    { from: "reservation", to: "inventory_level", label: "reserved_quantity increases" },
    { from: "reservation", to: "order_item", label: "line_item_id; order item reference" },
    { from: "order", to: "fulfillment", label: "stored one-to-many link" },
    { from: "fulfillment", to: "fulfillment_provider", label: "provider_id" },
    { from: "fulfillment_provider", to: "shipper", label: "create label / fulfill items" },
    { from: "shipper", to: "fulfillment", label: "tracking + labels" },
    { from: "fulfillment", to: "reservation", label: "consume stock; release reservation" },
    { from: "fulfillment", to: "inventory_level", label: "stocked and reserved decrease" },
    { from: "workflow", to: "event_bus", label: "domain events" },
    { from: "order", to: "event_bus", label: "order.placed" },
    { from: "payment", to: "event_bus", label: "payment.authorized" },
    { from: "fulfillment", to: "event_bus", label: "fulfillment created / shipped" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string; width: number; height: number }>;

export default sketch;
