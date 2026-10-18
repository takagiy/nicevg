// Blender 4.1 and 4.2 add-on and extension docs:
// https://docs.blender.org/manual/en/4.1/advanced/scripting/addon_tutorial.html
// https://developer.blender.org/docs/handbook/extensions/addon_dev_setup/
// https://developer.blender.org/docs/features/extensions/schema/1.0.0/
// https://www.blender.org/download/releases/4-2/
// Compares the old bundled add-on package with the 4.2 repository extension package.
// The install metadata changed, while `register()` still hooks an Operator into Blender's menu and type registry.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";
import { area, table, type ErNode } from "../styles/er.ts";

type Node = ErNode | {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "v41", class: "area", x: 25, y: 30, width: 510, height: 340, title: "Blender 4.1 — bundled legacy add-on" },
    { id: "legacy", class: "table", x: 45, y: 85, title: "my_addon.py / bl_info", columns: [["name", "str"], ["blender", "tuple"], ["category", "str"]], parent: "v41" },
    { id: "v42", class: "area", x: 600, y: 35, width: 560, height: 350, title: "Blender 4.2 — repository extension" },
    { id: "manifest", class: "table", x: 625, y: 85, title: "blender_manifest.toml", columns: [["id", "package id"], ["version", "semver"], ["type", "add-on"], ["blender_version_min", "4.2.0"]], parent: "v42" },
    { id: "repo", class: "table", x: 880, y: 100, title: "Extension repository entry", columns: [["id", "extension id"], ["archive_url", "package download"], ["version", "release"]], parent: "v42" },
    { id: "module", class: "box", x: 685, y: 265, label: "Installed package module\nbl_ext.<repository_module>.my_addon\nroot: __init__.py" },
    { id: "register", class: "box", x: 305, y: 260, label: "my_addon.register()\ncalled when add-on is enabled" },
    { id: "operator", class: "table", x: 255, y: 460, title: "ObjectMoveX(bpy.types.Operator)", columns: [["bl_idname", "object.move_x"], ["bl_label", "Move X by One"], ["bl_options", "REGISTER, UNDO"], ["execute(context)", "move selected scene objects"]] },
    { id: "types", class: "table", x: 650, y: 475, title: "bpy.types registry", columns: [["ObjectMoveX", "registered class"], ["operator id", "object.move_x"]] },
    { id: "menu", class: "table", x: 930, y: 475, title: "VIEW3D_MT_object", columns: [["draw callbacks", "ordered list"], ["menu_func", "layout.operator(id)"]] },
  ],
  edges: [
    { from: "legacy", to: "register", label: "4.1 enable imports module; reads bl_info" },
    { from: "manifest", to: "repo", label: "extension identity and release metadata" },
    { from: "repo", to: "module", label: "install package into repository" },
    { from: "module", to: "register", label: "4.2 enable imports package __init__.py" },
    { from: "register", to: "operator", label: "bpy.utils.register_class(ObjectMoveX)" },
    { from: "operator", to: "types", label: "class becomes available as bpy.ops.object.move_x" },
    { from: "register", to: "menu", label: "VIEW3D_MT_object.append(menu_func)" },
    { from: "menu", to: "operator", label: "menu_func adds operator button by bl_idname" },
    { from: "register", to: "menu", label: "disable: remove callback; unregister class in reverse order" },
  ],
  style: { area, table, box },
} satisfies Sketch<Node>;

export default sketch;
