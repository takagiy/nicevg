// Webpack's plugin API, following the plugin guide and compiler/compilation hook references.
// This traces an asset-producing plugin from the configuration's `plugins` array through
// compiler and compilation hooks into the compilation asset map and output emission.
// Sources: https://webpack.js.org/concepts/plugins/
//          https://webpack.js.org/api/compiler-hooks/
//          https://webpack.js.org/api/compilation-hooks/
//          https://webpack.js.org/api/compilation-object/
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
    { id: "config", class: "box", x: 30, y: 40, label: "webpack.config.js\nplugins: [ new ReportPlugin() ]" },
    { id: "plugin", class: "box", x: 350, y: 40, label: "ReportPlugin\napply(compiler)" },
    { id: "compiler", class: "box", x: 690, y: 40, label: "Compiler\nmain build engine; owns hook lifecycle" },
    { id: "compilation_hook", class: "box", x: 1030, y: 40, label: "compiler.hooks.compilation\nSyncHook(compilation, params)" },
    { id: "register", class: "box", x: 1030, y: 220, label: "Plugin callback\ncompilation => { ... }" },
    { id: "compilation", class: "box", x: 690, y: 220, label: "Compilation\ncurrent module graph, chunks and assets" },
    { id: "process_assets", class: "box", x: 350, y: 220, label: "compilation.hooks.processAssets\nAsyncSeriesHook; stage: ADDITIONAL" },
    { id: "asset_callback", class: "box", x: 30, y: 220, label: "Plugin asset callback\ncompilation.emitAsset(name, source)" },
    { id: "source", class: "box", x: 30, y: 410, label: "webpack-sources\nRawSource(report contents)" },
    { id: "assets", class: "box", x: 350, y: 410, label: "Compilation assets\nreport.json -> Source" },
    { id: "emit", class: "box", x: 690, y: 410, label: "Compiler emission\nprocessAssets finishes; emit assets" },
    { id: "done", class: "box", x: 1030, y: 410, label: "compiler.hooks.done\nSyncHook(stats); post-build reporting" },
  ],
  edges: [
    { from: "config", to: "plugin", label: "instantiate plugin in plugins array" },
    { from: "plugin", to: "compiler", label: "webpack calls apply(compiler)" },
    { from: "plugin", to: "compilation_hook", label: "tap('ReportPlugin', callback)" },
    { from: "compiler", to: "compilation_hook", label: "creates compilation; calls hook" },
    { from: "compilation_hook", to: "register", label: "compilation + compilationParams" },
    { from: "register", to: "compilation", label: "callback receives this build's object" },
    { from: "register", to: "process_assets", label: "tapPromise('ReportPlugin', { stage: ADDITIONAL }, fn)" },
    { from: "compilation", to: "process_assets", label: "runs asset processing stages" },
    { from: "process_assets", to: "asset_callback", label: "hook dispatch; assets yet to emit" },
    { from: "source", to: "asset_callback", label: "Source passed as asset content" },
    { from: "asset_callback", to: "assets", label: "emitAsset('report.json', source)" },
    { from: "assets", to: "emit", label: "asset map consumed by output stage" },
    { from: "emit", to: "done", label: "build completes with Stats" },
    { from: "plugin", to: "done", label: "also taps done(stats) in apply()" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
