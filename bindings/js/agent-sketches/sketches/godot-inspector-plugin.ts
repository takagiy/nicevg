// Godot Engine inspector plugin, following the stable inspector plugin guide.
// It traces plugin activation into the Inspector dock, where an EditorInspectorPlugin
// replaces an integer's default editor with an EditorProperty and sends edits back.
// Sources: https://docs.godotengine.org/en/stable/tutorials/plugins/editor/inspector_plugins.html
//          https://docs.godotengine.org/en/stable/tutorials/plugins/editor/making_plugins.html
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
    { id: "cfg", class: "box", x: 30, y: 40, label: "addons/my_inspector_plugin/plugin.cfg\nscript = plugin.gd" },
    { id: "plugin", class: "box", x: 350, y: 40, label: "plugin.gd : EditorPlugin\n_enter_tree(): preload(...).new()\nadd_inspector_plugin(plugin)\n_exit_tree(): remove_inspector_plugin(plugin)" },
    { id: "inspector", class: "box", x: 760, y: 40, label: "EditorInspector dock\nparses selected Object and its properties" },
    { id: "hook", class: "box", x: 760, y: 240, label: "my_inspector_plugin.gd\nEditorInspectorPlugin\n_can_handle(object)\n_parse_property(..., type, name, ...)" },
    { id: "default", class: "box", x: 1120, y: 240, label: "Default property editor\nkept when _parse_property returns false" },
    { id: "property", class: "box", x: 350, y: 430, label: "RandomIntEditor : EditorProperty\nadd_property_editor(name, editor)\nbutton emits_changed(property, value)" },
    { id: "object", class: "box", x: 30, y: 430, label: "Edited Object\ninteger property value" },
  ],
  edges: [
    { from: "cfg", to: "plugin", label: "editor loads declared script when enabled" },
    { from: "plugin", to: "hook", label: "instance registered with add_inspector_plugin()" },
    { from: "plugin", to: "hook", label: "disable: remove_inspector_plugin(instance)" },
    { from: "object", to: "inspector", label: "selected Object + property metadata" },
    { from: "inspector", to: "hook", label: "_can_handle(object); then _parse_property(type, name, ...)" },
    { from: "hook", to: "property", label: "integer: add_property_editor(name, RandomIntEditor.new()); return true" },
    { from: "hook", to: "default", label: "other type: return false; default editor remains" },
    { from: "property", to: "object", label: "emit_changed(property, value) updates edited property" },
    { from: "object", to: "property", label: "_update_property() reads current value" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
