// Open Food Facts Query schema: https://openfoodfacts.github.io/openfoodfacts-query/docs/explain-database-structure.html
// Product source fields: https://github.com/openfoodfacts/openfoodfacts-server/blob/main/html/data-fields.txt
// The diagram shows the query service's PostgreSQL records: staged product JSON, the product root,
// normalized facet rows and nutrients, country-specific popularity, and raw plus relational update history.

import type { Sketch } from "nicevg";
import { area, table, type ErNode } from "../styles/er.ts";

const nodes: ErNode[] = [
  { id: "imports", class: "area", x: 20, y: 20, width: 360, height: 170, title: "Import staging", fill: "#fff7ed" },
  { id: "history", class: "area", x: 400, y: 20, width: 580, height: 360, title: "Product update history", fill: "#fdf4ff" },
  { id: "catalog", class: "area", x: 20, y: 220, width: 960, height: 1400, title: "Query records", fill: "#f0f9ff" },

  { id: "product_temp", class: "table", parent: "imports", x: 50, y: 60, columns: [["raw_product", "jsonb"], ["process_id", "text"]] },

  { id: "product_update_event", class: "table", parent: "history", x: 430, y: 60, columns: [["id", "bigint", "PK"], ["message_id", "text"], ["timestamp", "timestamptz"], ["message", "jsonb"], ["collection_id", "integer"]] },
  { id: "product_update", class: "table", parent: "history", x: 690, y: 60, columns: [["id", "bigint", "PK"], ["product_id", "bigint", "FK"], ["event_id", "bigint", "FK"], ["revision", "integer"], ["update_type", "text"], ["contributor_id", "bigint", "FK"], ["updated_at", "timestamptz"]] },
  { id: "contributor", class: "table", parent: "history", x: 690, y: 240, columns: [["id", "bigint", "PK"], ["code", "text", "UQ"]] },

  { id: "product", class: "table", parent: "catalog", x: 50, y: 290, columns: [["id", "bigint", "PK"], ["code", "text", "UQ"], ["collection_id", "integer"], ["product_name", "text"], ["creator", "text"], ["created_t", "bigint"], ["last_modified_t", "bigint"], ["completeness", "real"], ["nutriscore_grade", "text"], ["ecoscore_grade", "text"], ["scans_n", "integer"]] },

  { id: "nutrient", class: "table", parent: "catalog", x: 400, y: 250, columns: [["id", "integer", "PK"], ["name", "text", "UQ"], ["unit", "text"]] },
  { id: "product_nutrient", class: "table", parent: "catalog", x: 660, y: 250, columns: [["product_id", "bigint", "FK"], ["nutrient_id", "integer", "FK"], ["value_100g", "real"], ["collection_id", "integer"]] },
  { id: "product_country", class: "table", parent: "catalog", x: 400, y: 410, columns: [["product_id", "bigint", "FK"], ["country", "text"], ["scans_n", "integer"], ["unique_scans_n", "integer"], ["collection_id", "integer"]] },

  { id: "product_categories_tag", class: "table", parent: "catalog", x: 50, y: 500, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_brands_tag", class: "table", parent: "catalog", x: 310, y: 500, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_labels_tag", class: "table", parent: "catalog", x: 570, y: 500, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_countries_tag", class: "table", parent: "catalog", x: 50, y: 650, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_ingredients_tag", class: "table", parent: "catalog", x: 310, y: 650, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_additives_tag", class: "table", parent: "catalog", x: 570, y: 650, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_allergens_tag", class: "table", parent: "catalog", x: 50, y: 790, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_traces_tag", class: "table", parent: "catalog", x: 310, y: 790, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_packaging_tag", class: "table", parent: "catalog", x: 570, y: 790, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },

  { id: "product_origins_tag", class: "table", parent: "catalog", x: 50, y: 930, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_manufacturing_places_tag", class: "table", parent: "catalog", x: 310, y: 930, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_emb_codes_tag", class: "table", parent: "catalog", x: 570, y: 930, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_purchase_places_tag", class: "table", parent: "catalog", x: 50, y: 1070, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_stores_tag", class: "table", parent: "catalog", x: 310, y: 1070, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_states_tag", class: "table", parent: "catalog", x: 570, y: 1070, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_nutrition_grades_tag", class: "table", parent: "catalog", x: 50, y: 1210, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_food_groups_tag", class: "table", parent: "catalog", x: 310, y: 1210, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_languages_tag", class: "table", parent: "catalog", x: 570, y: 1210, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_data_quality_errors_tag", class: "table", parent: "catalog", x: 50, y: 1350, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_data_quality_info_tag", class: "table", parent: "catalog", x: 310, y: 1350, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_data_quality_warnings_tag", class: "table", parent: "catalog", x: 570, y: 1350, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
  { id: "product_data_sources_tag", class: "table", parent: "catalog", x: 50, y: 1490, columns: [["product_id", "bigint", "FK"], ["tag", "text"], ["collection_id", "integer"]] },
];

const edges = [
  { from: "product_update.product_id", to: "product.id" },
  { from: "product_update.event_id", to: "product_update_event.id" },
  { from: "product_update.contributor_id", to: "contributor.id" },
  { from: "product_nutrient.product_id", to: "product.id" },
  { from: "product_nutrient.nutrient_id", to: "nutrient.id" },
  { from: "product_country.product_id", to: "product.id" },
  ...nodes.filter((node) => node.class === "table" && node.id.startsWith("product_") && node.id.endsWith("_tag")).map((node) => ({ from: `${node.id}.product_id`, to: "product.id" })),
];

export default { nodes, edges, style: { table, area } } satisfies Sketch<ErNode>;
