// pytest plugin hook registration and dispatch, following the plugin and hook guides and pytest's source.
// It traces a third-party pytest11 plugin into PytestPluginManager, then follows one
// pytest_collection_modifyitems call through wrapper, tryfirst, and trylast implementations.
// Sources: https://docs.pytest.org/en/latest/how-to/writing_plugins.html
//          https://docs.pytest.org/en/stable/how-to/writing_hook_functions.html
//          https://github.com/pytest-dev/pytest/blob/main/src/_pytest/config/__init__.py
//          https://github.com/pytest-dev/pytest/blob/main/src/_pytest/hookspec.py
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

interface PluginNode extends SketchNode {
  class: "meta" | "fn" | "core" | "impl";
  label: string;
}

const meta = ({ label }: PluginNode) => {
  const name = text(label, 13);
  const width = name.width + 28;
  const height = name.height + 20;
  return svg`<path data-bounds="true" d="M 10 0 H ${width - 10} L ${width} 10 V ${height - 10} L ${width - 10} ${height} H 10 L 0 ${height - 10} V 10 Z" fill="#fef3c7" stroke="#92400e"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
};

const fn = ({ label }: PluginNode) => {
  const name = text(label, 13);
  const width = name.width + 30;
  const height = name.height + 20;
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="${height / 2}" fill="#dbeafe" stroke="#1d4ed8"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
};

const core = ({ label }: PluginNode) => {
  const name = text(label, 13);
  const width = name.width + 32;
  const height = name.height + 24;
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="3" fill="#dcfce7" stroke="#166534" stroke-width="2"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
};

const impl = ({ label }: PluginNode) => {
  const name = text(label, 13);
  const width = name.width + 30;
  const height = name.height + 20;
  return svg`<path data-bounds="true" d="M 0 0 H ${width - 12} L ${width} ${height / 2} L ${width - 12} ${height} H 0 Z" fill="#fce7f3" stroke="#9d174d"/>${name.at({ cx: width / 2 - 5, cy: height / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "pyproject", class: "meta", x: 20, y: 20, label: "pyproject.toml\n[project.entry-points.pytest11]" },
    { id: "pytest11", class: "meta", x: 300, y: 20, label: "pytest11 entry point\nname → module" },
    { id: "loader", class: "fn", x: 570, y: 20, label: "load_setuptools_entrypoints(\"pytest11\")" },
    { id: "module", class: "meta", x: 880, y: 20, label: "imported plugin module\npytest_* functions" },
    { id: "pm", class: "core", x: 550, y: 180, label: "PytestPluginManager\n(Pluggy PluginManager)" },
    { id: "specs", class: "core", x: 220, y: 180, label: "_pytest.hookspec\npytest_collection_modifyitems(session, config, items)" },
    { id: "parse", class: "fn", x: 890, y: 180, label: "parse_hookimpl_opts\npytest_ prefix + hookimpl marks" },
    { id: "wrapper", class: "impl", x: 20, y: 360, label: "plugin wrapper\npytest_collection_modifyitems\n(wrapper=True)" },
    { id: "first", class: "impl", x: 340, y: 360, label: "plugin A\npytest_collection_modifyitems\n(tryfirst=True)" },
    { id: "last", class: "impl", x: 660, y: 360, label: "plugin B\npytest_collection_modifyitems\n(trylast=True)" },
    { id: "session", class: "core", x: 990, y: 360, label: "pytest collection\ncompleted" },
    { id: "hookcall", class: "fn", x: 950, y: 540, label: "config.hook.\npytest_collection_modifyitems" },
    { id: "items", class: "meta", x: 570, y: 550, label: "items: list[Item]\nshared mutable collection" },
    { id: "outcome", class: "meta", x: 70, y: 550, label: "hook result / exception\nwrapper resumes after yield" },
  ] satisfies PluginNode[],
  edges: [
    { from: "pyproject", to: "pytest11" },
    { from: "pytest11", to: "loader" },
    { from: "loader", to: "module", label: "import module" },
    { from: "module", to: "pm", label: "register(plugin)" },
    { from: "pm", to: "specs", label: "add_hookspecs" },
    { from: "module", to: "parse", label: "inspect pytest_* functions" },
    { from: "parse", to: "pm", label: "validated HookImpls" },
    { from: "pm", to: "wrapper", label: "registered hook implementation" },
    { from: "pm", to: "first" },
    { from: "pm", to: "last" },
    { from: "specs", to: "hookcall", label: "HookCaller for spec" },
    { from: "session", to: "hookcall", label: "invoke after collection" },
    { from: "hookcall", to: "wrapper", label: "same hook args" },
    { from: "hookcall", to: "first", label: "same hook args" },
    { from: "hookcall", to: "last", label: "same hook args" },
    { from: "hookcall", to: "items", label: "items=collected Item list" },
    { from: "wrapper", to: "first", label: "yield: wrappers enter first" },
    { from: "first", to: "last", label: "then tryfirst → trylast" },
    { from: "last", to: "outcome", label: "list of non-None results / error" },
    { from: "outcome", to: "wrapper", label: "resume at yield; post-process or raise" },
    { from: "wrapper", to: "hookcall", label: "final hook result" },
  ],
  style: { meta, fn, core, impl },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<PluginNode>;

export default sketch;
