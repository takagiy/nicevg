// Godot Engine GDExtension C++ example: https://docs.godotengine.org/en/4.6/tutorials/scripting/cpp/gdextension_cpp_example.html
// Load metadata, the shared-library entry point, staged initialization, ClassDB binding, and teardown.
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
    { id: "project", class: "box", x: 30, y: 40, label: "Godot project\nres://bin/example.gdextension" },
    { id: "descriptor", class: "box", x: 350, y: 40, label: "[configuration]\nentry_symbol = example_library_init\n[libraries]\nplatform -> shared library path" },
    { id: "loader", class: "box", x: 710, y: 40, label: "Godot GDExtension loader\nselect library for platform; load symbol" },
    { id: "entry", class: "box", x: 1080, y: 40, label: "register_types.cpp\nexample_library_init(get_proc_address, library, initialization)" },
    { id: "initobj", class: "box", x: 1080, y: 230, label: "GDExtensionBinding::InitObject\nregister_initializer(...)\nregister_terminator(...)\nminimum level = SCENE" },
    { id: "engine", class: "box", x: 710, y: 230, label: "Godot initialization levels\nCORE -> SERVERS -> SCENE -> EDITOR" },
    { id: "initialize", class: "box", x: 350, y: 230, label: "initialize_example_module(level)\nignore levels other than SCENE" },
    { id: "register", class: "box", x: 30, y: 230, label: "GDREGISTER_CLASS(GDExample)\nregister extension class with godot-cpp ClassDB" },
    { id: "classdb", class: "box", x: 30, y: 430, label: "Godot ClassDB\nGDExample available to scripts\nand Create Node dialog" },
    { id: "example", class: "box", x: 350, y: 430, label: "GDExample : Sprite2D\nGDCLASS + _bind_methods()\n_process(delta) updates position" },
    { id: "bind", class: "box", x: 710, y: 430, label: "GDExample::_bind_methods()\nexpose methods and properties" },
    { id: "terminate", class: "box", x: 1080, y: 430, label: "uninitialize_example_module(level)\nSCENE teardown callback\ncleanup extension-owned state" },
  ],
  edges: [
    { from: "project", to: "descriptor", label: "project references extension descriptor" },
    { from: "descriptor", to: "loader", label: "entry_symbol + platform library path" },
    { from: "loader", to: "entry", label: "loads binary; resolves named C entry symbol" },
    { from: "entry", to: "initobj", label: "construct InitObject from interface, library, initialization" },
    { from: "initobj", to: "engine", label: "init(): install callbacks and SCENE minimum level" },
    { from: "engine", to: "initialize", label: "at SCENE level: call registered initializer(level)" },
    { from: "initialize", to: "register", label: "GDREGISTER_CLASS(GDExample)" },
    { from: "register", to: "classdb", label: "ClassDB registers native extension class" },
    { from: "classdb", to: "example", label: "engine can instantiate registered type" },
    { from: "example", to: "bind", label: "class registration invokes _bind_methods()" },
    { from: "initobj", to: "terminate", label: "registered terminator on unload at SCENE level" },
    { from: "terminate", to: "classdb", label: "release extension-owned state before library unload" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
