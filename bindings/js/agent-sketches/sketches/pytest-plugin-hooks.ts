// pytest's plugin system, following the plugin authoring docs and Pluggy's hook model.
// This traces an installable pytest11 plugin from entry point discovery to hook registration,
// then a pytest_collect_file implementation returning a collector to pytest's collection tree.
// Sources: https://docs.pytest.org/en/latest/how-to/writing_plugins.html
//          https://docs.pytest.org/en/latest/reference/reference.html
//          https://pluggy.readthedocs.io/en/latest/
//          https://github.com/pytest-dev/pytest/blob/main/src/_pytest/config/__init__.py
//          https://github.com/pytest-dev/pluggy/blob/main/src/pluggy/_manager.py
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
    { id: "distribution", class: "box", x: 30, y: 30, label: "Plugin distribution\npyproject.toml: pytest11 = { yaml = 'pytest_yaml.plugin' }" },
    { id: "entrypoints", class: "box", x: 370, y: 30, label: "Installed entry point metadata\ngroup: pytest11\nname: yaml → value: pytest_yaml.plugin" },
    { id: "config", class: "box", x: 720, y: 30, label: "_pytest.config\nstartup parses -p no:name, loads builtins,\nthen explicit -p plugins" },
    { id: "autoload", class: "box", x: 720, y: 190, label: "Plugin auto-load\nunless PYTEST_DISABLE_PLUGIN_AUTOLOAD\nPluginManager.load_setuptools_entrypoints('pytest11')" },
    { id: "module", class: "box", x: 370, y: 190, label: "pytest_yaml.plugin module\npytest_collect_file(file_path, parent)\nreturns None or a custom Collector" },
    { id: "manager", class: "box", x: 30, y: 190, label: "pluggy.PluginManager\nregister(plugin, name)\nchecks hook implementations against specs" },
    { id: "spec", class: "box", x: 30, y: 360, label: "pytest hookspecs\n_pytest.hookspec.pytest_collect_file\nfile_path, parent → Collector | None" },
    { id: "caller", class: "box", x: 370, y: 360, label: "HookCaller: pytest_collect_file\ncollect hook implementations\n1:N call; firstresult semantics" },
    { id: "core", class: "box", x: 720, y: 360, label: "pytest collection\nDirectory.collect() visits file paths\nthen invokes pytest_collect_file" },
    { id: "file", class: "box", x: 720, y: 530, label: "test data file\nexample.yaml" },
    { id: "plugin_collector", class: "box", x: 370, y: 530, label: "YamlFile Collector\nplugin-created node for example.yaml\nimplements collect()" },
    { id: "items", class: "box", x: 30, y: 530, label: "pytest collection tree\nCollector yields child nodes / Items\nrunner later executes collected Items" },
  ],
  edges: [
    { from: "distribution", to: "entrypoints", label: "package metadata declares group + import target" },
    { from: "config", to: "autoload", label: "after builtins and -p plugins" },
    { from: "entrypoints", to: "autoload", label: "entry point for group pytest11" },
    { from: "autoload", to: "module", label: "loads entry point value (imports module)" },
    { from: "module", to: "manager", label: "module object registered as plugin" },
    { from: "spec", to: "manager", label: "hook specifications installed on manager" },
    { from: "manager", to: "caller", label: "registered pytest_collect_file implementations" },
    { from: "core", to: "caller", label: "file_path + parent arguments" },
    { from: "caller", to: "module", label: "dispatch hook implementation" },
    { from: "module", to: "caller", label: "None (keep trying) or first Collector result" },
    { from: "file", to: "core", label: "path encountered while traversing directory" },
    { from: "caller", to: "plugin_collector", label: "plugin returns YamlFile for .yaml" },
    { from: "plugin_collector", to: "items", label: "collect() yields pytest nodes / test Items" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
