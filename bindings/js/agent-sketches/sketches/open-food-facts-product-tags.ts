// Open Food Facts product records and the normalized tags used to classify them.
// Sources: https://openfoodfacts.github.io/documentation/docs/Product-Opener/schemas/schemas/product/
// https://github.com/openfoodfacts/openfoodfacts-server/blob/main/html/data-fields.txt
// The sketch shows the product's barcode-keyed data, tag arrays, and the taxonomy entries those tags name.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "record" | "boundary";
  x: number;
  y: number;
  label: string;
  parent?: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "product-record", class: "record", x: 220, y: 100, label: "Product\nPK code (barcode) · id · schema_version\nproduct_name · generic_name · quantity\nbrands · countries_tags · product_type\ncreated_t · last_modified_t" },
    { id: "classification", class: "boundary", x: 20, y: 20, width: 700, height: 530, label: "Product fields and normalized taxonomy tags", fill: "#f0f9ff" },
    { id: "category-tags", class: "record", x: 40, y: 140, parent: "classification", label: "categories_tags[]\nnormalized category tag strings\n(e.g. en:breakfast-cereals)" },
    { id: "category-taxonomy", class: "record", x: 40, y: 350, parent: "classification", label: "Category taxonomy entry\nPK tag id · localized names\nparents · children" },
    { id: "ingredient-tags", class: "record", x: 480, y: 140, parent: "classification", label: "ingredients_tags[]\nnormalized ingredient tag strings\n(e.g. en:sugar)" },
    { id: "ingredient-taxonomy", class: "record", x: 480, y: 350, parent: "classification", label: "Ingredient taxonomy entry\nPK tag id · localized names\nparents · children" },
  ] satisfies Node[],
  edges: [
    { from: "product-record", to: "category-tags", label: "categories_tags array" },
    { from: "category-tags", to: "category-taxonomy", label: "each tag resolves to taxonomy id" },
    { from: "product-record", to: "ingredient-tags", label: "ingredients_tags array" },
    { from: "ingredient-tags", to: "ingredient-taxonomy", label: "each tag resolves to taxonomy id" },
    { from: "category-taxonomy", to: "category-taxonomy", label: "parent / child hierarchy" },
    { from: "ingredient-taxonomy", to: "ingredient-taxonomy", label: "parent / child hierarchy" },
  ],
  style: { record: box, boundary: box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
