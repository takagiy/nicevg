// Home Assistant config entry migration and integration hook-in lifecycle, based on:
// https://developers.home-assistant.io/docs/core/integration/config_flow/
// https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/runtime-data/
// https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/config-entry-unloading/
// This follows an integration upgrade that changes stored config data, migrates each entry,
// and then hooks the integration's platforms into Home Assistant Core.
import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

interface Node extends SketchNode {
  class: "box";
  label: string;
}

const sketch = {
  nodes: [
    { id: "maintainer", class: "box", x: 30, y: 25, label: "Integration maintainer" },
    { id: "flow", class: "box", x: 270, y: 20, label: "ConfigFlow\nVERSION = 2" },
    { id: "package", class: "box", x: 30, y: 170, label: "Integration package\ncustom_components/example" },
    { id: "entry", class: "box", x: 280, y: 170, label: "ConfigEntry\nversion=1 · data={ip_address}" },
    { id: "manager", class: "box", x: 550, y: 170, label: "HomeAssistant\nConfigEntries" },
    { id: "startup", class: "box", x: 800, y: 170, label: "Core startup\nload config entries" },
    { id: "migrate", class: "box", x: 800, y: 330, label: "async_migrate_entry" },
    { id: "olddata", class: "box", x: 550, y: 330, label: "entry.data\n{ip_address: ...}" },
    { id: "transform", class: "box", x: 300, y: 330, label: "Copy data; rename\nip_address → host" },
    { id: "newdata", class: "box", x: 30, y: 330, label: "new_data\n{host: ...}" },
    { id: "update", class: "box", x: 30, y: 490, label: "async_update_entry\ndata + version=2" },
    { id: "success", class: "box", x: 300, y: 490, label: "return True\nmigration succeeded" },
    { id: "error", class: "box", x: 550, y: 490, label: "Migration error\nsetup stops" },
    { id: "repair", class: "box", x: 800, y: 490, label: "Repair flow\nresolve cause" },
    { id: "operator", class: "box", x: 1050, y: 490, label: "Home Assistant user" },
    { id: "retry", class: "box", x: 1050, y: 330, label: "async_retry_migration\n(entry_id)" },
    { id: "setup", class: "box", x: 1050, y: 170, label: "async_setup_entry" },
    { id: "runtime", class: "box", x: 1320, y: 170, label: "entry.runtime_data\nclient / coordinator" },
    { id: "forward", class: "box", x: 1320, y: 330, label: "async_forward_entry_setups\n(entry, PLATFORMS)" },
    { id: "platform", class: "box", x: 1580, y: 330, label: "Platform module\nsensor.py" },
    { id: "entities", class: "box", x: 1580, y: 490, label: "async_setup_entry\nasync_add_entities" },
    { id: "unload", class: "box", x: 1320, y: 650, label: "async_unload_entry" },
    { id: "unloadplatform", class: "box", x: 1580, y: 650, label: "async_unload_platforms\n(entry, PLATFORMS)" },
  ] satisfies Node[],
  edges: [
    { from: "maintainer", to: "flow", label: "bump VERSION" },
    { from: "maintainer", to: "package", label: "implement migration hook" },
    { from: "flow", to: "manager", label: "integration version metadata" },
    { from: "package", to: "manager", label: "integration code" },
    { from: "entry", to: "manager", label: "stored entry" },
    { from: "startup", to: "manager", label: "async_setup" },
    { from: "manager", to: "migrate", label: "older version: call hook before setup" },
    { from: "migrate", to: "olddata", label: "read config_entry.data" },
    { from: "olddata", to: "transform", label: "version 1 shape" },
    { from: "transform", to: "newdata", label: "renamed host field" },
    { from: "newdata", to: "update", label: "data=new_data" },
    { from: "update", to: "entry", label: "persist data + version" },
    { from: "migrate", to: "success", label: "return True" },
    { from: "migrate", to: "error", label: "False / ConfigEntryError" },
    { from: "error", to: "repair", label: "migration error state" },
    { from: "operator", to: "repair", label: "fix reported cause" },
    { from: "repair", to: "retry", label: "retry after repair" },
    { from: "retry", to: "manager", label: "entry_id" },
    { from: "success", to: "setup", label: "continue config entry setup" },
    { from: "manager", to: "setup", label: "async_setup_entry(hass, entry)" },
    { from: "setup", to: "runtime", label: "store runtime objects" },
    { from: "runtime", to: "forward", label: "entry.runtime_data ready" },
    { from: "forward", to: "platform", label: "PLATFORMS" },
    { from: "platform", to: "entities", label: "forward config entry" },
    { from: "entities", to: "runtime", label: "use runtime_data" },
    { from: "manager", to: "unload", label: "entry unload requested" },
    { from: "unload", to: "unloadplatform", label: "unload platforms" },
    { from: "unloadplatform", to: "manager", label: "unload result" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
