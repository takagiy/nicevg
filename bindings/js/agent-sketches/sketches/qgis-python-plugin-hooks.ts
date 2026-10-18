// QGIS Python plugins. Sources: https://github.com/qgis/QGIS/blob/master/python/utils.py
// and https://docs.qgis.org/3.10/en/docs/pyqgis_developer_cookbook/plugins/plugins.html.
// This sketch follows a package through QGIS's Python loader into the plugin instance,
// its GUI hooks, and the cleanup/error paths that remove registrations and modules.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type N = { id: string; class: "box"; x: number; y: number; label: string };
const nodes: N[] = [
  { id: "plugin-manager", class: "box", x: 40, y: 40, label: "QGIS Plugin Manager" },
  { id: "enabled", class: "box", x: 40, y: 150, label: "installed + enabled packageName" },
  { id: "utils", class: "box", x: 260, y: 150, label: "python/utils.py" },
  { id: "import", class: "box", x: 480, y: 40, label: "_import(packageName)" },
  { id: "package", class: "box", x: 700, y: 40, label: "plugin package\n__init__.py + metadata.txt" },
  { id: "sysmodules", class: "box", x: 700, y: 150, label: "sys.modules[packageName]" },
  { id: "plugin-modules", class: "box", x: 700, y: 270, label: "_plugin_modules[packageName]" },
  { id: "start", class: "box", x: 480, y: 270, label: "_startPlugin(packageName)" },
  { id: "active-check", class: "box", x: 260, y: 270, label: "already active?\nreturn False" },
  { id: "settings", class: "box", x: 920, y: 150, label: "QgsSettingsTree\ncreatePluginTreeNode" },
  { id: "factory", class: "box", x: 920, y: 40, label: "package.classFactory(iface)" },
  { id: "iface", class: "box", x: 1160, y: 40, label: "QgisInterface (iface)" },
  { id: "instance", class: "box", x: 1160, y: 150, label: "plugins[packageName]\nplugin instance" },
  { id: "startgui", class: "box", x: 1380, y: 150, label: "startPlugin()" },
  { id: "initgui", class: "box", x: 1600, y: 40, label: "plugin.initGui()" },
  { id: "action", class: "box", x: 1820, y: 40, label: "QAction / triggered.connect(run)" },
  { id: "toolbar", class: "box", x: 2040, y: 40, label: "iface.addToolBarIcon(action)" },
  { id: "menu", class: "box", x: 2040, y: 150, label: "iface.addPluginToMenu(menu, action)" },
  { id: "canvas", class: "box", x: 1820, y: 270, label: "iface.mapCanvas()" },
  { id: "signal", class: "box", x: 2040, y: 270, label: "renderComplete.connect(renderTest)" },
  { id: "run", class: "box", x: 1820, y: 390, label: "plugin.run() callback" },
  { id: "render-test", class: "box", x: 2040, y: 390, label: "plugin.renderTest(painter) callback" },
  { id: "active", class: "box", x: 1600, y: 270, label: "active_plugins + plugin_times" },
  { id: "unload", class: "box", x: 1380, y: 390, label: "unloadPlugin(packageName)" },
  { id: "plugin-unload", class: "box", x: 1160, y: 390, label: "plugin.unload()" },
  { id: "remove-menu", class: "box", x: 920, y: 390, label: "removePluginMenu(action)" },
  { id: "remove-toolbar", class: "box", x: 700, y: 390, label: "removeToolBarIcon(action)" },
  { id: "disconnect", class: "box", x: 480, y: 390, label: "renderComplete.disconnect(renderTest)" },
  { id: "unregister-settings", class: "box", x: 260, y: 390, label: "unregisterPluginTreeNode()" },
  { id: "module-cleanup", class: "box", x: 40, y: 390, label: "_unloadPluginModules(packageName)" },
  { id: "qcleanup", class: "box", x: 40, y: 510, label: "qCleanupResources() when present" },
  { id: "sys-path", class: "box", x: 260, y: 510, label: "remove package paths from sys.path" },
  { id: "sys-delete", class: "box", x: 480, y: 510, label: "delete plugin modules from sys.modules" },
  { id: "class-error", class: "box", x: 920, y: 510, label: "classFactory exception" },
  { id: "gui-error", class: "box", x: 1160, y: 510, label: "initGui exception" },
  { id: "exception", class: "box", x: 1380, y: 510, label: "showException(messagebar, Critical)" },
  { id: "loader-return", class: "box", x: 1600, y: 510, label: "loader reports False / load failure" },
  { id: "success", class: "box", x: 1820, y: 510, label: "return True; plugin active" },
];

const edges = ([
  ["plugin-manager", "enabled"], ["enabled", "utils"], ["utils", "import"], ["import", "package"],
  ["package", "sysmodules"], ["sysmodules", "plugin-modules"], ["utils", "start"], ["start", "active-check"],
  ["active-check", "sysmodules"], ["sysmodules", "settings"], ["settings", "factory"], ["factory", "iface"],
  ["factory", "instance"], ["instance", "startgui"], ["startgui", "initgui"], ["initgui", "action"],
  ["action", "toolbar"], ["action", "menu"], ["initgui", "canvas"], ["canvas", "signal"],
  ["action", "run"], ["signal", "render-test"], ["initgui", "active"], ["active", "success"],
  ["plugin-manager", "unload"], ["unload", "plugin-unload"], ["plugin-unload", "remove-menu"],
  ["plugin-unload", "remove-toolbar"], ["plugin-unload", "disconnect"], ["unload", "unregister-settings"],
  ["unload", "module-cleanup"], ["module-cleanup", "qcleanup"], ["module-cleanup", "sys-path"],
  ["module-cleanup", "sys-delete"], ["factory", "class-error"], ["class-error", "unregister-settings"],
  ["class-error", "module-cleanup"], ["class-error", "exception"], ["initgui", "gui-error"],
  ["gui-error", "module-cleanup"], ["gui-error", "exception"], ["exception", "loader-return"],
 ] as const).map(([from, to]) => ({ from, to }));

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif", "font-size": 12 } } satisfies Sketch<N>;
