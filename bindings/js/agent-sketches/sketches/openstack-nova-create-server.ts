// OpenStack Nova server creation, based on https://docs.openstack.org/nova/latest/admin/architecture.html,
// https://docs.openstack.org/nova/latest/admin/cells.html, https://docs.openstack.org/nova/latest/reference/scheduling.html,
// https://docs.openstack.org/nova/latest/admin/scheduling.html, and https://docs.openstack.org/nova/latest/install/get-started-compute.html.
// Follows an image-backed server request through Keystone, nova-api, the super conductor, Placement,
// a target cell and hypervisor, including allocation retries, build failure rescheduling and the API reply.

import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "person" | "step" | "service" | "system" | "store" | "boundary";
type Node = SketchNode & { class: Kind; label: string; width?: number; height?: number; fill?: string; title?: string };

const node = (id: string, cls: Kind, x: number, y: number, label: string, width = 176, height = 64, parent?: string): Node =>
  ({ id, class: cls, x, y, label, width, height, ...(parent ? { parent } : {}) });

const nodes: Node[] = [
  node("control", "boundary", 20, 20, "OpenStack control plane", 2750, 900),
  node("cell", "boundary", 970, 390, "Cell 1 — cell message bus + DB", 1650, 560, "control"),
  node("user", "person", 25, 250, "Cloud user / operator\nrequests a server", 150, 78),
  node("client", "step", 205, 250, "OpenStackClient\nserver create", 160, 72),
  node("keystone", "system", 205, 100, "Keystone Identity\nissue scoped token", 172, 68),
  node("http", "step", 400, 250, "Compute REST API\nPOST /servers", 165, 72),
  node("authn", "step", 405, 95, "Validate token +\nproject scope", 158, 68),
  node("policy", "step", 585, 95, "oslo.policy\ncompute:create", 150, 68),
  node("validate", "step", 585, 250, "Request validation\nimage, flavor, network", 178, 72),
  node("quota", "step", 780, 250, "Quota + request checks\ninstance count", 172, 72),
  node("image", "system", 780, 105, "Glance Image API\nimage metadata", 172, 68),
  node("network", "system", 780, 480, "Neutron API\nrequested network", 172, 68),
  node("create", "step", 970, 250, "Compute API\ncreate instance objects", 174, 72),
  node("api-db", "store", 990, 105, "API DB\ninstance mapping +\nbuild request", 176, 82),
  node("cell0", "store", 1195, 105, "Cell0 DB\nunscheduled instances", 165, 78),
  node("rpc1", "system", 990, 470, "oslo.messaging\nAPI queue", 166, 70),
  node("super", "service", 1185, 250, "nova-conductor\n(super conductor)", 180, 75),
  node("reqspec", "step", 1185, 400, "BuildRequest + RequestSpec\nresource requirements", 190, 78),
  node("schedrpc", "step", 1395, 250, "Scheduler RPC call\nselect_destinations", 174, 74),
  node("scheduler", "service", 1595, 250, "nova-scheduler\nFilterScheduler", 170, 74),
  node("placement", "system", 1595, 95, "Placement API\nproviders + allocations", 190, 80),
  node("candidates", "step", 1595, 415, "Allocation candidates\nprovider summaries", 185, 72),
  node("filters", "step", 1795, 250, "HostState filters\nthen weighers", 170, 72),
  node("claim", "step", 1795, 415, "Claim allocation\ntry ranked hosts", 170, 72),
  node("race", "step", 1975, 415, "Claim conflict?\ntry next candidate", 168, 72),
  node("alternates", "step", 1975, 250, "Selected host +\nsame-cell alternates", 176, 72),
  node("cellrpc", "system", 1185, 570, "Cell 1 oslo.messaging\nRPC queue", 185, 72, "cell"),
  node("conductor", "service", 1400, 570, "nova-conductor\ncell workflow", 180, 72, "cell"),
  node("db", "store", 1400, 735, "Cell DB\nInstance + BuildRequest", 180, 76, "cell"),
  node("build", "step", 1600, 570, "build_instances\ntry selected host", 170, 72, "cell"),
  node("compute-rpc", "step", 1790, 570, "Compute RPC\nbuild_and_run_instance", 190, 76, "cell"),
  node("compute", "service", 1990, 570, "nova-compute\nComputeManager", 175, 72, "cell"),
  node("image-download", "step", 1990, 735, "Fetch image\nGlance image data", 165, 72, "cell"),
  node("virt", "service", 2180, 570, "libvirt driver\nspawn guest", 170, 72, "cell"),
  node("hypervisor", "system", 2180, 735, "KVM / QEMU\ninstance process", 170, 72, "cell"),
  node("neutron-bind", "step", 2180, 420, "allocate + bind ports\nNeutron RPC", 175, 76, "cell"),
  node("success", "step", 2390, 570, "Instance ACTIVE\nupdate state + events", 185, 76, "cell"),
  node("failed", "step", 2390, 735, "Build failed\nunclaim resources", 170, 72, "cell"),
  node("retry", "step", 2590, 735, "Try alternate\nclaim, then build", 172, 72, "cell"),
  node("error", "step", 2590, 420, "NoValidHost / build error\ninstance ERROR", 195, 76),
  node("reply", "step", 780, 650, "Compute API response\nserver id + links", 176, 72),
  node("httpreply", "person", 400, 650, "Client receives\nHTTP 202 + server", 170, 76),
];

const rect = (n: Node, color: string, rounded = 8, dashed = "") => svg`<rect data-bounds="true" width="${n.width ?? 176}" height="${n.height ?? 64}" rx="${rounded}" fill="${n.fill ?? color}" stroke="#475569" stroke-width="1.4" stroke-dasharray="${dashed}"/>${text(n.label, 12).at({ cx: (n.width ?? 176) / 2, cy: (n.height ?? 64) / 2 })}`;
const person = (n: Node) => svg`<rect data-bounds="true" width="${n.width}" height="${n.height}" rx="30" fill="#fff7ed" stroke="#9a3412" stroke-width="1.5"/>${text(n.label, 12).at({ cx: (n.width ?? 176) / 2, cy: (n.height ?? 64) / 2 })}`;
const store = (n: Node) => svg`<rect data-bounds="true" width="${n.width}" height="${n.height}" rx="16" fill="#f0fdf4" stroke="#166534" stroke-width="1.5"/>${text(n.label, 12).at({ cx: (n.width ?? 176) / 2, cy: (n.height ?? 64) / 2 })}`;
const boundary = (n: Node) => svg`<rect data-bounds="true" width="${n.width}" height="${n.height}" rx="12" fill="#f8fafc" fill-opacity="0.35" stroke="#94a3b8" stroke-dasharray="7 5"/>${text(n.label, 14).at({ left: 12, top: 8 })}`;

const edges = [
  { from: "user", to: "client" }, { from: "client", to: "keystone" }, { from: "keystone", to: "authn" },
  { from: "client", to: "http" }, { from: "authn", to: "http" }, { from: "http", to: "policy" },
  { from: "policy", to: "validate" }, { from: "validate", to: "image" }, { from: "image", to: "validate" },
  { from: "validate", to: "network" }, { from: "network", to: "validate" }, { from: "validate", to: "quota" },
  { from: "quota", to: "create" }, { from: "create", to: "api-db" }, { from: "create", to: "cell0" },
  { from: "create", to: "rpc1" }, { from: "rpc1", to: "super" }, { from: "super", to: "reqspec" },
  { from: "reqspec", to: "schedrpc" }, { from: "schedrpc", to: "scheduler" },
  { from: "scheduler", to: "placement" }, { from: "placement", to: "candidates" },
  { from: "candidates", to: "filters" }, { from: "filters", to: "claim" },
  { from: "claim", to: "placement" }, { from: "claim", to: "race" }, { from: "race", to: "claim" },
  { from: "claim", to: "alternates" }, { from: "alternates", to: "schedrpc" },
  { from: "scheduler", to: "super" }, { from: "super", to: "cellrpc" },
  { from: "cellrpc", to: "conductor" }, { from: "conductor", to: "db" },
  { from: "conductor", to: "build" }, { from: "build", to: "compute-rpc" },
  { from: "compute-rpc", to: "compute" }, { from: "compute", to: "image-download" },
  { from: "image-download", to: "image" }, { from: "compute", to: "neutron-bind" },
  { from: "neutron-bind", to: "network" }, { from: "compute", to: "virt" },
  { from: "virt", to: "hypervisor" }, { from: "hypervisor", to: "success" },
  { from: "success", to: "db" }, { from: "compute", to: "failed" },
  { from: "failed", to: "placement" }, { from: "failed", to: "retry" },
  { from: "retry", to: "compute-rpc" }, { from: "retry", to: "error" },
  { from: "error", to: "db" }, { from: "rpc1", to: "reply" },
  { from: "reply", to: "httpreply" }, { from: "httpreply", to: "user" },
];

export default {
  nodes,
  edges,
  style: {
    person,
    step: (n) => rect(n, "#fff", 5),
    service: (n) => rect(n, "#eff6ff", 10),
    system: (n) => rect(n, "#fefce8", 2),
    store,
    boundary,
  },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
