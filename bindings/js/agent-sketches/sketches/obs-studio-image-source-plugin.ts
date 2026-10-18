// OBS Studio. Sources: https://github.com/obsproject/obs-studio/blob/master/docs/sphinx/plugins.rst,
// https://github.com/obsproject/obs-studio/blob/master/plugins/image-source/image-source.c,
// https://github.com/obsproject/obs-studio/blob/master/libobs/obs-source.c.
// The sketch follows the built-in image-source module from module discovery and registration
// into libobs source creation, callbacks, image loading, rendering, settings updates and cleanup.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type N = SketchNode & { class: "unit" | "hook" | "data" | "boundary"; label: string; width?: number; height?: number };
const unit = ({ label, width = 180, height = 54 }: N) => svg`<rect data-bounds="true" width="${width}" height="${height}" rx="4" fill="#e2e8f0" stroke="#475569"/><path d="M0 13h${width}" stroke="#94a3b8"/>${text(label, 12).at({ cx: width / 2, cy: height / 2 + 7 })}`;
const hook = ({ label, width = 176, height = 46 }: N) => svg`<path data-bounds="true" d="M0 0h${width - 12}l12 ${height / 2}-12 ${height / 2}H0z" fill="#fef3c7" stroke="#92400e"/>${text(label, 11).at({ cx: width / 2 - 5, cy: height / 2 })}`;
const data = ({ label, width = 174, height = 48 }: N) => svg`<ellipse data-bounds="true" cx="${width / 2}" cy="${height / 2}" rx="${width / 2}" ry="${height / 2}" fill="#dcfce7" stroke="#166534"/>${text(label, 11).at({ cx: width / 2, cy: height / 2 })}`;
const boundary = ({ label, width = 430, height = 520 }: N) => svg`<rect data-bounds="true" width="${width}" height="${height}" rx="9" fill="#f8fafc" stroke="#64748b" stroke-dasharray="6 4"/>${text(label, 13).at({ left: 12, top: 8 })}`;

const nodes: N[] = [
  { id: "host", class: "boundary", x: 20, y: 20, width: 470, height: 570, label: "OBS process · libobs" },
  { id: "module-boundary", class: "boundary", x: 530, y: 20, width: 500, height: 570, label: "image-source module" },
  { id: "ui", class: "unit", x: 40, y: 70, label: "OBS UI / scene loader" },
  { id: "paths", class: "data", x: 40, y: 165, label: "module search paths" },
  { id: "find", class: "hook", x: 265, y: 70, label: "obs_find_modules" },
  { id: "module-file", class: "data", x: 275, y: 165, label: "image-source module file" },
  { id: "open", class: "hook", x: 275, y: 250, label: "open module + symbols" },
  { id: "init", class: "hook", x: 40, y: 250, label: "obs_init_module" },
  { id: "declare", class: "hook", x: 560, y: 70, label: "OBS_DECLARE_MODULE" },
  { id: "load", class: "hook", x: 790, y: 70, label: "obs_module_load()" },
  { id: "locale", class: "unit", x: 560, y: 150, label: "default locale · en-US" },
  { id: "info", class: "data", x: 790, y: 150, label: "image_source_info\nid: image_source · INPUT · VIDEO" },
  { id: "register", class: "hook", x: 790, y: 245, label: "obs_register_source(info)" },
  { id: "registry", class: "data", x: 560, y: 245, label: "libobs source_types registry" },
  { id: "source-menu", class: "unit", x: 40, y: 350, label: "Add Source / persisted scene" },
  { id: "settings", class: "data", x: 40, y: 445, label: "obs_data_t settings\nfile · unload · linear_alpha" },
  { id: "create-api", class: "hook", x: 275, y: 350, label: "obs_source_create(id, name, settings)" },
  { id: "lookup", class: "hook", x: 560, y: 350, label: "get_source_info(id)" },
  { id: "create-cb", class: "hook", x: 790, y: 350, label: "image_source_create" },
  { id: "context", class: "data", x: 560, y: 435, label: "image_source context\nsource · file · image · flags" },
  { id: "update-cb", class: "hook", x: 790, y: 435, label: "image_source_update" },
  { id: "image-load", class: "unit", x: 560, y: 520, label: "decode file → GPU texture" },
  { id: "props-cb", class: "hook", x: 40, y: 520, label: "get_properties → file / toggles" },
  { id: "defaults", class: "hook", x: 275, y: 520, label: "get_defaults → bool defaults" },
  { id: "show", class: "hook", x: 1060, y: 70, label: "show / hide" },
  { id: "activate", class: "hook", x: 1060, y: 150, label: "activate" },
  { id: "render", class: "hook", x: 1060, y: 245, label: "video_render(effect)" },
  { id: "dimensions", class: "hook", x: 1060, y: 350, label: "get_width / get_height" },
  { id: "destroy", class: "hook", x: 1060, y: 435, label: "image_source_destroy" },
  { id: "unload", class: "hook", x: 1060, y: 520, label: "unload texture + free context" },
  { id: "file", class: "data", x: 1300, y: 70, label: "image file path" },
  { id: "gpu", class: "data", x: 1300, y: 245, label: "gs_texture_t" },
  { id: "scene", class: "unit", x: 1300, y: 435, label: "scene item / compositor" },
];

const sketch = {
  nodes,
  edges: [
    { from: "ui", to: "paths", label: "obs_add_module_path" }, { from: "paths", to: "find" },
    { from: "find", to: "module-file", label: "module info" }, { from: "module-file", to: "open" },
    { from: "open", to: "init", label: "obs_module_t" }, { from: "init", to: "declare" },
    { from: "declare", to: "load", label: "exported entry point" }, { from: "load", to: "locale" },
    { from: "load", to: "info" }, { from: "info", to: "register" }, { from: "register", to: "registry", label: "id → obs_source_info" },
    { from: "source-menu", to: "settings" }, { from: "source-menu", to: "create-api", label: "id = image_source" },
    { from: "settings", to: "create-api" }, { from: "create-api", to: "lookup" }, { from: "registry", to: "lookup" },
    { from: "lookup", to: "create-cb", label: "info.create" }, { from: "settings", to: "create-cb" },
    { from: "create-cb", to: "context", label: "private data" }, { from: "create-cb", to: "update-cb", label: "initial update" },
    { from: "info", to: "update-cb", label: "info.update" }, { from: "settings", to: "update-cb" },
    { from: "update-cb", to: "file", label: "settings['file']" }, { from: "update-cb", to: "image-load", label: "persistent or showing" },
    { from: "image-load", to: "gpu" }, { from: "props-cb", to: "settings", label: "properties edit → update" },
    { from: "info", to: "props-cb", label: "info.get_properties" }, { from: "info", to: "defaults", label: "info.get_defaults" },
    { from: "info", to: "show", label: "info.show / hide" }, { from: "show", to: "image-load", label: "load when shown; unload when hidden" },
    { from: "info", to: "activate", label: "info.activate" }, { from: "info", to: "render", label: "info.video_render" },
    { from: "gpu", to: "render", label: "texture" }, { from: "render", to: "scene", label: "draw image" },
    { from: "info", to: "dimensions", label: "info.get_width / get_height" }, { from: "context", to: "dimensions" },
    { from: "info", to: "destroy", label: "info.destroy" }, { from: "destroy", to: "unload" },
    { from: "unload", to: "gpu", label: "free GPU resource" }, { from: "unload", to: "context", label: "free file + struct" },
  ],
  style: { unit, hook, data, boundary },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<N>;

export default sketch;
