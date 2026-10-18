// pytest plugin loading and hook dispatch, following pytest's plugin and hook-function guides.
// It shows startup discovery into Pytest's pluggy PluginManager, then how collection hook
// implementations wrap or modify the shared item list before collection proceeds.
// Sources: https://docs.pytest.org/en/stable/how-to/writing_plugins.html
//          https://docs.pytest.org/en/stable/how-to/writing_hook_functions.html
//          https://docs.pytest.org/en/stable/reference/reference.html
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string };

const sketch = {
  nodes: [
    { id: "pytest", class: "box", x: 360, y: 20, label: "pytest startup\npytest.main(args)" },
    { id: "builtin", class: "box", x: 20, y: 150, label: "Builtin plugins\n_pytest.* modules" },
    { id: "cli-plugin", class: "box", x: 300, y: 150, label: "-p name\nexplicit command-line plugin" },
    { id: "entrypoint", class: "box", x: 580, y: 150, label: "pytest11 entry points\ninstalled third-party plugins" },
    { id: "env-plugin", class: "box", x: 860, y: 150, label: "PYTEST_PLUGINS\nenvironment module names" },
    { id: "conftest", class: "box", x: 1140, y: 150, label: "conftest.py\ninitial test-path directories" },
    { id: "manager", class: "box", x: 470, y: 300, label: "PytestPluginManager\nregister plugin objects; parse pytest_* methods" },
    { id: "hookimpls", class: "box", x: 470, y: 450, label: "Pluggy HookImpl list\npytest_collection_modifyitems implementations" },
    { id: "spec", class: "box", x: 30, y: 450, label: "pytest_collection_modifyitems spec\nitems: list[Item], config: Config\nmutates collected item list" },
    { id: "caller", class: "box", x: 470, y: 610, label: "Pluggy multicall\n1:N implementations for this hook" },
    { id: "wrapper", class: "box", x: 30, y: 770, label: "Plugin hook wrapper\n@pytest.hookimpl(hookwrapper=True)\nyield; post-process Result" },
    { id: "tryfirst", class: "box", x: 470, y: 770, label: "Plugin A\n@pytest.hookimpl(tryfirst=True)\nreorder / filter items" },
    { id: "trylast", class: "box", x: 900, y: 770, label: "Plugin B\n@pytest.hookimpl(trylast=True)\nmodify items after earlier hooks" },
  ],
  edges: [
    { from: "pytest", to: "builtin", label: "load builtins first" },
    { from: "pytest", to: "cli-plugin", label: "-p name, unless blocked by -p no:name" },
    { from: "pytest", to: "entrypoint", label: "discover installed pytest11 entry points" },
    { from: "pytest", to: "env-plugin", label: "load named modules" },
    { from: "pytest", to: "conftest", label: "discover along test path; then pytest_plugins recursively" },
    { from: "builtin", to: "manager", label: "plugin module" },
    { from: "cli-plugin", to: "manager", label: "plugin module" },
    { from: "entrypoint", to: "manager", label: "import entry point target" },
    { from: "env-plugin", to: "manager", label: "import module" },
    { from: "conftest", to: "manager", label: "register directory-scoped plugin" },
    { from: "spec", to: "caller", label: "hook name + args: items, config" },
    { from: "manager", to: "hookimpls", label: "registered plugin methods matching spec" },
    { from: "hookimpls", to: "caller", label: "ordered implementations + options" },
    { from: "caller", to: "wrapper", label: "wrapper enters before regular hooks" },
    { from: "caller", to: "tryfirst", label: "runs early among regular hooks" },
    { from: "tryfirst", to: "trylast", label: "shared mutable items list" },
    { from: "trylast", to: "caller", label: "hook result collection" },
    { from: "caller", to: "wrapper", label: "yield resumes with Result; inspect or force result" },
    { from: "wrapper", to: "spec", label: "final collection hook result; collection continues" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
