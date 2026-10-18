// Visual Studio Code command extension hook, following the extension anatomy and command guide.
// It traces a contributed command from package metadata through lazy activation and extension-host
// registration to the workbench command registry, including the callback crossing the RPC boundary.
// Sources: https://code.visualstudio.com/api/get-started/extension-anatomy
//          https://code.visualstudio.com/api/extension-guides/command
//          https://code.visualstudio.com/api/advanced-topics/extension-host
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/api/common/extHostCommands.ts
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/api/common/extHost.protocol.ts
//          https://github.com/microsoft/vscode/blob/main/src/vs/workbench/services/extensions/common/abstractExtensionService.ts
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const sketch = {
  nodes: [
    { id: "package", class: "box", x: 30, y: 30, label: "Extension package\npackage.json", width: 210, height: 68 },
    { id: "commands-contribution", class: "box", x: 30, y: 150, label: "contributes.commands\ncommand ID + title", width: 230, height: 72 },
    { id: "activation", class: "box", x: 300, y: 150, label: "Command activation event\nimplicit for contributed commands\n(VS Code 1.74+)", width: 270, height: 82 },
    { id: "palette", class: "box", x: 30, y: 300, label: "Command Palette\nvisible title", width: 210, height: 62 },
    { id: "command-service", class: "box", x: 300, y: 300, label: "Workbench command invocation\ncommand ID", width: 270, height: 70 },
    { id: "extension-service", class: "box", x: 620, y: 300, label: "ExtensionService\nactivate interested extensions", width: 250, height: 70 },
    { id: "host-manager", class: "box", x: 620, y: 430, label: "Extension host manager\nactivate(extensionId, reason)", width: 260, height: 72 },
    { id: "extension-host", class: "box", x: 960, y: 410, label: "Extension Host process\nloads extension module", width: 240, height: 72 },
    { id: "main-module", class: "box", x: 950, y: 550, label: "Extension main module\nexports activate(context)", width: 250, height: 72 },
    { id: "register-api", class: "box", x: 640, y: 570, label: "vscode.commands.registerCommand(id, handler)\nreturns Disposable", width: 300, height: 82 },
    { id: "ext-host-commands", class: "box", x: 300, y: 570, label: "ExtHostCommands\nstores handler by command ID", width: 280, height: 72 },
    { id: "rpc-main", class: "box", x: 300, y: 720, label: "MainThreadCommands\nRPC $registerCommand(id)", width: 280, height: 72 },
    { id: "registry", class: "box", x: 30, y: 720, label: "CommandsRegistry\nworkbench dispatches registered ID", width: 240, height: 72 },
  ],
  edges: [
    { from: "package", to: "commands-contribution", label: "manifest contribution" },
    { from: "package", to: "activation", label: "extension description" },
    { from: "commands-contribution", to: "palette", label: "title + command ID" },
    { from: "commands-contribution", to: "activation", label: "activation mapping" },
    { from: "palette", to: "command-service", label: "user invokes command ID" },
    { from: "activation", to: "extension-service", label: "matching activation event" },
    { from: "command-service", to: "extension-service", label: "command activation request" },
    { from: "extension-service", to: "host-manager", label: "extension ID + reason" },
    { from: "host-manager", to: "extension-host", label: "host RPC activation" },
    { from: "extension-host", to: "main-module", label: "load entry point" },
    { from: "main-module", to: "register-api", label: "activate(context) registers handler" },
    { from: "register-api", to: "ext-host-commands", label: "id + callback" },
    { from: "ext-host-commands", to: "rpc-main", label: "$registerCommand(id) over RPC" },
    { from: "rpc-main", to: "registry", label: "install dispatch entry" },
    { from: "registry", to: "ext-host-commands", label: "execute handler; callback result returns over RPC" },
  ],
  style: { box },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string; width: number; height: number }>;

export default sketch;
