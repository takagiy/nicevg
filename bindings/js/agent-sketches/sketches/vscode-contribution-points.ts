// Visual Studio Code extension contribution points.
// Sources: https://code.visualstudio.com/api/references/contribution-points
//          https://code.visualstudio.com/api/get-started/extension-anatomy
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/services/extensions/common/extensionsRegistry.ts
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/api/common/configurationExtensionPoint.ts
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/api/browser/viewsExtensionPoint.ts
// Shows how manifest declarations are matched to registered extension points, validated and handed
// to their workbench handlers, alongside the separate runtime activation and provider registration path.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  parent?: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "extension", class: "box", x: 30, y: 160, width: 230, height: 90, label: "Installed extension\npackage.json + extension code", fill: "#f1f5f9" },
    { id: "manifest", class: "box", x: 320, y: 60, width: 260, height: 90, label: "Extension manifest\ncontributes.commands\nmenus + views", fill: "#dbeafe" },
    { id: "registry", class: "box", x: 650, y: 60, width: 300, height: 90, label: "ExtensionsRegistry\nregisterExtensionPoint(name, schema)\npoint handler + dependencies", fill: "#e0e7ff" },
    { id: "json-registry", class: "box", x: 1010, y: 60, width: 260, height: 90, label: "JSON contribution registry\nmanifest schema for contributes", fill: "#fef3c7" },
    { id: "validate", class: "box", x: 650, y: 210, width: 300, height: 90, label: "Extension point processing\ncollect contributions\nvalidate schema + dependencies", fill: "#fef3c7" },
    { id: "commands-point", class: "box", x: 320, y: 340, width: 250, height: 90, label: "commands extension point\ncommand id, title, enablement", fill: "#dcfce7" },
    { id: "menus-point", class: "box", x: 650, y: 340, width: 250, height: 90, label: "menus extension point\nlocation, command, when, group", fill: "#dcfce7" },
    { id: "views-point", class: "box", x: 980, y: 340, width: 250, height: 90, label: "views extension point\ncontainer id, view id, name", fill: "#dcfce7" },
    { id: "command-ui", class: "box", x: 320, y: 500, width: 250, height: 75, label: "Command Palette / menu item\nuser selects command", fill: "#fce7f3" },
    { id: "activation", class: "box", x: 650, y: 500, width: 270, height: 75, label: "ExtensionService\nactivate onCommand:<id> / onView:<id>", fill: "#ffedd5" },
    { id: "host", class: "box", x: 980, y: 500, width: 250, height: 75, label: "Extension Host\nload extension entry point", fill: "#ffedd5" },
    { id: "api", class: "box", x: 980, y: 640, width: 250, height: 90, label: "Extension API registrations\nregisterCommand(handler)\nregisterTreeDataProvider(viewId, provider)", fill: "#cffafe" },
    { id: "callback", class: "box", x: 650, y: 640, width: 270, height: 90, label: "Workbench dispatch\ncommand handler callback\nTreeDataProvider.getTreeItem / getChildren", fill: "#fce7f3" },
    { id: "workbench", class: "box", x: 30, y: 500, width: 230, height: 90, label: "Workbench UI\nregistered menus + views", fill: "#f1f5f9" },
  ],
  edges: [
    { from: "extension", to: "manifest", label: "extension description" },
    { from: "registry", to: "json-registry", label: "register manifest schema" },
    { from: "manifest", to: "validate", label: "contributes object" },
    { from: "registry", to: "validate", label: "known points + handlers" },
    { from: "json-registry", to: "validate", label: "schema validation" },
    { from: "validate", to: "commands-point", label: "commands contribution" },
    { from: "validate", to: "menus-point", label: "menus contribution" },
    { from: "validate", to: "views-point", label: "views contribution" },
    { from: "commands-point", to: "command-ui", label: "title + command id" },
    { from: "menus-point", to: "command-ui", label: "when context + command id" },
    { from: "commands-point", to: "activation", label: "onCommand:<id> activation event" },
    { from: "views-point", to: "activation", label: "onView:<id> when view is opened" },
    { from: "views-point", to: "workbench", label: "register view in container" },
    { from: "command-ui", to: "activation", label: "invoke command id" },
    { from: "activation", to: "host", label: "activate extension" },
    { from: "extension", to: "host", label: "main entry point" },
    { from: "host", to: "api", label: "extension calls vscode API" },
    { from: "api", to: "callback", label: "command handler + view provider callbacks" },
    { from: "callback", to: "workbench", label: "command result / tree items" },
    { from: "workbench", to: "callback", label: "getTreeItem(element), getChildren(element)" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
