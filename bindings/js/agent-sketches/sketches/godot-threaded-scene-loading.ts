// Godot Engine background scene loading, based on
// https://docs.godotengine.org/en/4.4/tutorials/io/background_loading.html and
// https://docs.godotengine.org/en/4.4/classes/class_resourceloader.html.
// It follows a scene path through a threaded ResourceLoader request, status polling and retrieval,
// then instantiation into the active scene tree; retrieval can still block if called too early.

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
    { id: "scene-path", class: "box", x: 30, y: 180, label: '"Enemy.tscn"\nres:// scene path' },
    { id: "request", class: "box", x: 270, y: 180, label: "ResourceLoader\nload_threaded_request(path)" },
    { id: "background-load", class: "box", x: 560, y: 70, label: "Background threads\nload scene resource" },
    { id: "status", class: "box", x: 560, y: 300, label: "_process()\nload_threaded_get_status(path, progress)" },
    { id: "get-resource", class: "box", x: 870, y: 180, label: "load_threaded_get(path)\nreturns PackedScene\nmay block if not ready" },
    { id: "instantiate", class: "box", x: 1170, y: 180, label: "PackedScene.instantiate()\nnew enemy Node" },
    { id: "scene-tree", class: "box", x: 1450, y: 180, label: "add_child(enemy)\nactive scene tree" },
  ],
  edges: [
    { from: "scene-path", to: "request", label: "path" },
    { from: "request", to: "background-load", label: "queue threaded load(path)" },
    { from: "background-load", to: "status", label: "load status; progress fraction" },
    { from: "status", to: "status", label: "poll on later frames until loaded" },
    { from: "status", to: "get-resource", label: "when needed; status is loaded" },
    { from: "get-resource", to: "instantiate", label: "PackedScene resource" },
    { from: "instantiate", to: "scene-tree", label: "Node" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
