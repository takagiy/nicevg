// Saleor commerce records, based on https://docs.saleor.io/developer/products/overview,
// https://docs.saleor.io/developer/checkout/overview, and the Saleor models in
// https://github.com/saleor/saleor/tree/main/saleor. Catalog variants acquire channel prices,
// become checkout and order lines, consume stock through reservations and allocations, and connect
// to fulfillment and payment transactions.
import type { Sketch } from "nicevg";
import { area, table, type Column, type ErNode } from "../styles/er.ts";

const groups = [
  { id: "catalog", title: "Catalog & pricing", x: 20, y: 20, width: 650, height: 630, fill: "#eff6ff" },
  { id: "checkout", title: "Checkout", x: 690, y: 20, width: 550, height: 420, fill: "#f0fdf4" },
  { id: "inventory", title: "Inventory", x: 1260, y: 20, width: 590, height: 570, fill: "#fff7ed" },
  { id: "orders", title: "Orders & fulfillment", x: 690, y: 470, width: 550, height: 620, fill: "#fdf2f8" },
  { id: "payments", title: "Payments", x: 1260, y: 610, width: 590, height: 480, fill: "#f5f3ff" },
  { id: "identity", title: "Customers & addresses", x: 20, y: 680, width: 650, height: 410, fill: "#f8fafc" },
] as const;

const records = [
  ["channel", "catalog", 50, 70, [["id", "int", "PK"], ["slug", "varchar", "UQ"], ["currency_code", "char"], ["is_active", "bool"]]],
  ["product_type", "catalog", 50, 230, [["id", "int", "PK"], ["name", "varchar"], ["has_variants", "bool"]]],
  ["category", "catalog", 50, 390, [["id", "int", "PK"], ["parent_id", "int", "FK"], ["name", "varchar"], ["slug", "varchar"]]],
  ["product", "catalog", 270, 70, [["id", "int", "PK"], ["product_type_id", "int", "FK"], ["category_id", "int", "FK"], ["name", "varchar"], ["slug", "varchar"]]],
  ["product_variant", "catalog", 270, 250, [["id", "int", "PK"], ["product_id", "int", "FK"], ["sku", "varchar"], ["name", "varchar"], ["track_inventory", "bool"]]],
  ["product_variant_channel_listing", "catalog", 470, 420, [["id", "int", "PK"], ["variant_id", "int", "FK"], ["channel_id", "int", "FK"], ["currency", "char"], ["price_amount", "decimal"], ["cost_price_amount", "decimal"]]],
  ["promotion", "catalog", 470, 70, [["id", "int", "PK"], ["name", "varchar"], ["type", "varchar"], ["start_date", "datetime"]]],
  ["promotion_rule", "catalog", 470, 230, [["id", "int", "PK"], ["promotion_id", "int", "FK"], ["name", "varchar"], ["catalogue_predicate", "json"]]],

  ["checkout", "checkout", 720, 70, [["token", "uuid", "PK"], ["channel_id", "int", "FK"], ["user_id", "int", "FK"], ["billing_address_id", "int", "FK"], ["shipping_address_id", "int", "FK"], ["email", "varchar"]]],
  ["checkout_line", "checkout", 970, 70, [["id", "int", "PK"], ["checkout_token", "uuid", "FK"], ["variant_id", "int", "FK"], ["quantity", "int"], ["price_amount", "decimal"]]],
  ["checkout_delivery", "checkout", 720, 270, [["id", "int", "PK"], ["checkout_token", "uuid", "FK"], ["method_name", "varchar"], ["shipping_method_id", "int", "FK"]]],
  ["checkout_voucher", "checkout", 970, 270, [["id", "int", "PK"], ["checkout_token", "uuid", "FK"], ["voucher_code", "varchar"]]],

  ["warehouse", "inventory", 1290, 70, [["id", "int", "PK"], ["name", "varchar"], ["slug", "varchar"], ["company_name", "varchar"]]],
  ["channel_warehouse", "inventory", 1510, 70, [["id", "int", "PK"], ["channel_id", "int", "FK"], ["warehouse_id", "int", "FK"], ["sort_order", "int"]]],
  ["stock", "inventory", 1290, 250, [["id", "int", "PK"], ["warehouse_id", "int", "FK"], ["product_variant_id", "int", "FK"], ["quantity", "int"], ["quantity_allocated", "int"]]],
  ["reservation", "inventory", 1510, 250, [["id", "int", "PK"], ["checkout_line_id", "int", "FK"], ["stock_id", "int", "FK"], ["quantity_reserved", "int"], ["reserved_until", "datetime"]]],
  ["allocation", "inventory", 1510, 430, [["id", "int", "PK"], ["order_line_id", "uuid", "FK"], ["stock_id", "int", "FK"], ["quantity_allocated", "int"]]],

  ["order", "orders", 720, 520, [["id", "uuid", "PK"], ["user_id", "int", "FK"], ["channel_id", "int", "FK"], ["billing_address_id", "int", "FK"], ["shipping_address_id", "int", "FK"], ["number", "int"], ["status", "varchar"], ["total_gross_amount", "decimal"]]],
  ["order_line", "orders", 970, 520, [["id", "uuid", "PK"], ["order_id", "uuid", "FK"], ["variant_id", "int", "FK"], ["product_name", "varchar"], ["product_sku", "varchar"], ["quantity", "int"], ["quantity_fulfilled", "int"]]],
  ["fulfillment", "orders", 720, 790, [["id", "int", "PK"], ["order_id", "uuid", "FK"], ["fulfillment_order", "int"], ["status", "varchar"], ["tracking_number", "varchar"]]],
  ["fulfillment_line", "orders", 970, 790, [["id", "int", "PK"], ["fulfillment_id", "int", "FK"], ["order_line_id", "uuid", "FK"], ["stock_id", "int", "FK"], ["quantity", "int"]]],
  ["order_event", "orders", 720, 960, [["id", "int", "PK"], ["order_id", "uuid", "FK"], ["user_id", "int", "FK"], ["type", "varchar"], ["date", "datetime"], ["parameters", "json"]]],
  ["order_granted_refund", "orders", 970, 960, [["id", "int", "PK"], ["order_id", "uuid", "FK"], ["transaction_item_id", "uuid", "FK"], ["amount_value", "decimal"], ["status", "varchar"]]],

  ["payment", "payments", 1290, 650, [["id", "uuid", "PK"], ["order_id", "uuid", "FK"], ["gateway", "varchar"], ["is_active", "bool"], ["charge_status", "varchar"]]],
  ["transaction_item", "payments", 1510, 650, [["id", "uuid", "PK"], ["payment_id", "uuid", "FK"], ["name", "varchar"], ["psp_reference", "varchar"], ["amount_value", "decimal"], ["currency", "char"]]],
  ["transaction_event", "payments", 1510, 850, [["id", "uuid", "PK"], ["transaction_id", "uuid", "FK"], ["type", "varchar"], ["amount_value", "decimal"], ["created_at", "datetime"]]],

  ["user", "identity", 50, 730, [["id", "int", "PK"], ["email", "varchar"], ["first_name", "varchar"], ["last_name", "varchar"], ["is_active", "bool"]]],
  ["address", "identity", 270, 730, [["id", "int", "PK"], ["user_id", "int", "FK"], ["first_name", "varchar"], ["last_name", "varchar"], ["street_address_1", "varchar"], ["city", "varchar"], ["country", "char"]]],
  ["user_address", "identity", 470, 730, [["id", "int", "PK"], ["user_id", "int", "FK"], ["address_id", "int", "FK"], ["is_default_shipping", "bool"], ["is_default_billing", "bool"]]],
  ["user_event", "identity", 50, 930, [["id", "int", "PK"], ["user_id", "int", "FK"], ["type", "varchar"], ["date", "datetime"], ["parameters", "json"]]],
] as const;

const nodes: ErNode[] = [
  ...groups.map((g) => ({ ...g, class: "area" as const })),
  ...records.map(([id, parent, x, y, columns]) => ({ id, parent, x, y, class: "table" as const, columns: columns.map(([name, type, key]) => [name, type, key] as Column) })),
];

const edges = [
  ["product.product_type_id", "product_type.id"], ["product.category_id", "category.id"], ["category.parent_id", "category.id"],
  ["product_variant.product_id", "product.id"], ["product_variant_channel_listing.variant_id", "product_variant.id"], ["product_variant_channel_listing.channel_id", "channel.id"],
  ["promotion_rule.promotion_id", "promotion.id"],
  ["checkout.channel_id", "channel.id"], ["checkout.user_id", "user.id"], ["checkout.billing_address_id", "address.id"], ["checkout.shipping_address_id", "address.id"],
  ["checkout_line.checkout_token", "checkout.token"], ["checkout_line.variant_id", "product_variant.id"], ["checkout_delivery.checkout_token", "checkout.token"], ["checkout_voucher.checkout_token", "checkout.token"],
  ["channel_warehouse.channel_id", "channel.id"], ["channel_warehouse.warehouse_id", "warehouse.id"], ["stock.warehouse_id", "warehouse.id"], ["stock.product_variant_id", "product_variant.id"],
  ["reservation.checkout_line_id", "checkout_line.id"], ["reservation.stock_id", "stock.id"], ["allocation.order_line_id", "order_line.id"], ["allocation.stock_id", "stock.id"],
  ["order.user_id", "user.id"], ["order.channel_id", "channel.id"], ["order.billing_address_id", "address.id"], ["order.shipping_address_id", "address.id"],
  ["order_line.order_id", "order.id"], ["order_line.variant_id", "product_variant.id"], ["fulfillment.order_id", "order.id"], ["fulfillment_line.fulfillment_id", "fulfillment.id"], ["fulfillment_line.order_line_id", "order_line.id"], ["fulfillment_line.stock_id", "stock.id"],
  ["order_event.order_id", "order.id"], ["order_event.user_id", "user.id"], ["order_granted_refund.order_id", "order.id"], ["order_granted_refund.transaction_item_id", "transaction_item.id"],
  ["payment.order_id", "order.id"], ["transaction_item.payment_id", "payment.id"], ["transaction_event.transaction_id", "transaction_item.id"],
  ["address.user_id", "user.id"], ["user_address.user_id", "user.id"], ["user_address.address_id", "address.id"], ["user_event.user_id", "user.id"],
].map(([from, to]) => ({ from: from!, to: to! }));

export default { nodes, edges, style: { table, area } } satisfies Sketch<ErNode>;
