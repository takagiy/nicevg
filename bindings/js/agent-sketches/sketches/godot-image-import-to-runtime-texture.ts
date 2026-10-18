// Godot Engine image import and runtime loading.
// URLs: https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/import_process.html
//       https://docs.godotengine.org/en/4.4/classes/class_resourceloader.html
//       https://docs.godotengine.org/en/stable/classes/class_resourceformatloader.html
// This follows a project image from its source file through editor import and the generated
// imported resource, then through ResourceLoader into a runtime texture consumed by a Sprite2D.

import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
};

const nodes: Node[] = [
  { id: "source", class: "box", x: 20, y: 100, label: "res://art/hero.png\nsource image" },
  { id: "file_system", class: "box", x: 210, y: 100, label: "Editor FileSystem\nasset detected" },
  { id: "source_hash", class: "box", x: 410, y: 100, label: "MD5 checksum\nchanged?" },
  { id: "import_preset", class: "box", x: 600, y: 30, label: "hero.png.import\nimport configuration" },
  { id: "editor_importer", class: "box", x: 600, y: 170, label: "Editor importer\nimage → texture data" },
  { id: "imported_files", class: "box", x: 830, y: 80, label: ".godot/imported/\n*.ctex + *.md5" },
  { id: "api_call", class: "box", x: 20, y: 370, label: "load(\"res://art/hero.png\")\nResourceLoader request" },
  { id: "path_remap", class: "box", x: 250, y: 370, label: "Import remap\noriginal path → imported path" },
  { id: "cache", class: "box", x: 500, y: 320, label: "Resource cache\nreuse cached object" },
  { id: "format_loader", class: "box", x: 500, y: 450, label: "ResourceFormatLoader\nhandles imported format" },
  { id: "texture", class: "box", x: 760, y: 390, label: "Texture2D\nloaded Resource" },
  { id: "sprite", class: "box", x: 970, y: 390, label: "Sprite2D\ntexture property" },
  { id: "frame", class: "box", x: 1170, y: 390, label: "Rendered image\nin viewport" },
];

const sketch = {
  nodes,
  edges: [
    { from: "source", to: "file_system", label: "project file added" },
    { from: "file_system", to: "source_hash", label: "track source checksum" },
    { from: "source_hash", to: "editor_importer", label: "new or changed: import / reimport" },
    { from: "import_preset", to: "editor_importer", label: "import options" },
    { from: "editor_importer", to: "imported_files", label: "converted texture + checksum" },
    { from: "editor_importer", to: "import_preset", label: "write/update sidecar metadata" },
    { from: "api_call", to: "path_remap", label: "res:// path" },
    { from: "imported_files", to: "path_remap", label: "sidecar points to generated artifact" },
    { from: "path_remap", to: "cache", label: "resolved resource identity" },
    { from: "cache", to: "texture", label: "hit: return cached Resource" },
    { from: "cache", to: "format_loader", label: "miss: load storage data" },
    { from: "imported_files", to: "format_loader", label: "imported .ctex data" },
    { from: "format_loader", to: "texture", label: "decode and construct Texture2D" },
    { from: "texture", to: "cache", label: "store loaded resource" },
    { from: "texture", to: "sprite", label: "assign Texture2D" },
    { from: "sprite", to: "frame", label: "draw texture" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
