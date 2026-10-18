// Home Assistant Core registry records, based on:
// https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/device_registry.py
// https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/entity_registry.py
// https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/area_registry.py
// https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/floor_registry.py
// https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/label_registry.py
// https://developers.home-assistant.io/docs/device_registry_index/
// https://developers.home-assistant.io/docs/entity_registry_index/
// This sketch maps the persistent registries that associate integration configuration,
// devices, entities, and the areas/floors/labels used to organize them.
import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = { id: string; class: "box"; x: number; y: number; parent?: string; label: string; fill?: string };
const nodes: Node[] = [
  { id: "configuration", class: "box", x: 20, y: 20, label: "Configuration ownership", fill: "#eff6ff" },
  { id: "core-config-entries", class: "box", x: 40, y: 90, parent: "configuration", label: "core.config_entries\nConfigEntry: entry_id (PK), domain,\ntitle, source, data, options, state" },
  { id: "config-subentries", class: "box", x: 350, y: 90, parent: "configuration", label: "ConfigSubentry\nsubentry_id (PK), config_entry_id,\nsubentry_type, title, data, options" },
  { id: "config-entry-options", class: "box", x: 350, y: 220, parent: "configuration", label: "ConfigEntry.options\nIntegration-specific settings map" },
  { id: "device-registry", class: "box", x: 700, y: 20, label: "Device registry · core.device_registry", fill: "#f0fdf4" },
  { id: "device-entry", class: "box", x: 720, y: 90, parent: "device-registry", label: "DeviceEntry\nid (PK), config_entry_id, config_subentry_id,\narea_id, parent_device_id, via_device_id,\nname, manufacturer, model, serial_number,\nidentifiers, connections, disabled_by" },
  { id: "device-identifiers", class: "box", x: 720, y: 270, parent: "device-registry", label: "identifiers[]\n(domain, identifier) tuples\nmatch device within its config entry" },
  { id: "device-connections", class: "box", x: 1010, y: 270, parent: "device-registry", label: "connections[]\n(type, identifier) tuples\ne.g. mac, bluetooth, zigbee" },
  { id: "entity-registry", class: "box", x: 1320, y: 20, label: "Entity registry · core.entity_registry", fill: "#fdf4ff" },
  { id: "registry-entry", class: "box", x: 1340, y: 90, parent: "entity-registry", label: "RegistryEntry\nid (PK), entity_id, platform, unique_id,\nconfig_entry_id, config_subentry_id, device_id,\narea_id, name, original_name, icon, device_class,\nunit_of_measurement, disabled_by, hidden_by" },
  { id: "entity-categories", class: "box", x: 1340, y: 280, parent: "entity-registry", label: "categories\nentity category assignments" },
  { id: "entity-labels", class: "box", x: 1620, y: 280, parent: "entity-registry", label: "labels[]\nlabel IDs assigned to entity" },
  { id: "entity-options", class: "box", x: 1340, y: 400, parent: "entity-registry", label: "options\nper-entity integration options" },
  { id: "entity-aliases", class: "box", x: 1620, y: 400, parent: "entity-registry", label: "aliases_v2[]\nordered aliases; may include computed name" },
  { id: "deleted-entities", class: "box", x: 1620, y: 90, parent: "entity-registry", label: "deleted_entities[]\nretained removed-entity identity/settings" },
  { id: "entity-settings", class: "box", x: 1620, y: 180, parent: "entity-registry", label: "settings\nentity_id_parts naming configuration" },
  { id: "location", class: "box", x: 20, y: 430, label: "Location registry", fill: "#fff7ed" },
  { id: "area-registry", class: "box", x: 40, y: 500, parent: "location", label: "core.area_registry · AreaEntry\nid (PK), name, aliases, floor_id,\ntemperature_entity_id, humidity_entity_id,\nicon, picture, labels, created_at, modified_at" },
  { id: "floor-registry", class: "box", x: 350, y: 500, parent: "location", label: "core.floor_registry · FloorEntry\nid (PK), name, level, icon, aliases" },
  { id: "label-registry", class: "box", x: 40, y: 670, parent: "location", label: "core.labels · LabelEntry\nid (PK), name, color, description, icon" },
  { id: "device-labels", class: "box", x: 350, y: 670, parent: "location", label: "DeviceEntry.labels[]\nassigned label IDs" },
  { id: "area-labels", class: "box", x: 650, y: 670, parent: "location", label: "AreaEntry.labels[]\nassigned label IDs" },
  { id: "floor-areas", class: "box", x: 650, y: 500, parent: "location", label: "Floor → areas\nAreaEntry.floor_id references FloorEntry.id" },
  { id: "orphaned", class: "box", x: 1010, y: 500, label: "Orphan retention", fill: "#f8fafc" },
  { id: "orphaned-device", class: "box", x: 1030, y: 570, parent: "orphaned", label: "orphaned_timestamp\nDeviceEntry: retained after config removal" },
  { id: "orphaned-entity", class: "box", x: 1340, y: 570, parent: "orphaned", label: "orphaned_timestamp\nRegistryEntry: retained after integration removal" },
  { id: "area-assignment", class: "box", x: 720, y: 430, label: "Area assignment\nDevices and entities can each carry area_id", fill: "#fff7ed" },
  { id: "device-topology", class: "box", x: 1010, y: 90, label: "Device topology\nparent_device_id = logical child\nvia_device_id = routing hub", fill: "#f0fdf4" },
  { id: "unique-entity-key", class: "box", x: 1620, y: 500, label: "Lookup key\n(platform, domain, unique_id)\nkeeps entity_id stable", fill: "#fdf4ff" },
];

const edges = [
  { from: "config-subentries", to: "core-config-entries", label: "config_entry_id" },
  { from: "config-entry-options", to: "core-config-entries", label: "options map" },
  { from: "device-entry", to: "core-config-entries", label: "config_entry_id" },
  { from: "device-entry", to: "config-subentries", label: "config_subentry_id (optional)" },
  { from: "registry-entry", to: "core-config-entries", label: "config_entry_id" },
  { from: "registry-entry", to: "config-subentries", label: "config_subentry_id (optional)" },
  { from: "registry-entry", to: "device-entry", label: "device_id (optional)" },
  { from: "device-identifiers", to: "device-entry", label: "identifies / matches" },
  { from: "device-connections", to: "device-entry", label: "identifies / matches" },
  { from: "device-topology", to: "device-entry", label: "parent_device_id, via_device_id" },
  { from: "device-entry", to: "area-registry", label: "area_id" },
  { from: "registry-entry", to: "area-registry", label: "area_id override" },
  { from: "area-registry", to: "floor-registry", label: "floor_id" },
  { from: "floor-areas", to: "area-registry", label: "groups areas" },
  { from: "device-labels", to: "label-registry", label: "label IDs" },
  { from: "area-labels", to: "label-registry", label: "label IDs" },
  { from: "entity-labels", to: "label-registry", label: "label IDs" },
  { from: "area-registry", to: "registry-entry", label: "temperature / humidity entity IDs" },
  { from: "entity-categories", to: "registry-entry", label: "category metadata" },
  { from: "entity-options", to: "registry-entry", label: "stored with entity" },
  { from: "entity-aliases", to: "registry-entry", label: "stored with entity" },
  { from: "deleted-entities", to: "registry-entry", label: "removed entry snapshot" },
  { from: "entity-settings", to: "entity-registry", label: "registry-wide setting" },
  { from: "unique-entity-key", to: "registry-entry", label: "lookup tuple" },
  { from: "orphaned-device", to: "device-entry", label: "timestamp field" },
  { from: "orphaned-entity", to: "registry-entry", label: "timestamp field" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
