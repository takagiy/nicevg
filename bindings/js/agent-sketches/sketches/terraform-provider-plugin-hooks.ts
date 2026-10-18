// Terraform's provider plugin boundary, following HashiCorp's plugin, provider server and RPC docs.
// This traces a protocol v6 provider from process startup and schema discovery through configuring
// a provider client, planning one resource change, and applying it against the remote API.
// Sources: https://developer.hashicorp.com/terraform/plugin/how-terraform-works
//          https://developer.hashicorp.com/terraform/plugin/terraform-plugin-protocol
//          https://developer.hashicorp.com/terraform/plugin/framework/provider-servers
//          https://developer.hashicorp.com/terraform/plugin/framework/internals/rpcs
//          https://developer.hashicorp.com/terraform/plugin/framework/providers
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  parent?: string;
};

const sketch = {
  nodes: [
    { id: "config", class: "box", x: 30, y: 40, label: "Terraform configuration\nrequired_providers + resource block" },
    { id: "core", class: "box", x: 390, y: 40, label: "Terraform CLI / Core\nresource graph, state, plan executor" },
    { id: "binary", class: "box", x: 780, y: 40, label: "Provider executable\nseparate OS process" },
    { id: "server", class: "box", x: 1120, y: 40, label: "providerserver.Serve\nplugin handshake + gRPC server" },
    { id: "framework", class: "box", x: 1120, y: 210, label: "terraform-plugin-framework\nproto6server → fwserver dispatch" },
    { id: "provider", class: "box", x: 780, y: 210, label: "provider.Provider implementation\nSchema() · Configure() · Resources()" },
    { id: "resource_factory", class: "box", x: 390, y: 210, label: "Resources() factories\nreturn fresh resource.Resource instances" },
    { id: "resource", class: "box", x: 30, y: 210, label: "resource.Resource implementation\nSchema() · Read() · ModifyPlan()\nCreate() / Update() / Delete()" },
    { id: "schema_rpc", class: "box", x: 30, y: 390, label: "GetProviderSchema RPC\nprovider + resource + data source schemas" },
    { id: "schema_result", class: "box", x: 390, y: 390, label: "GetProviderSchema response\nTerraform learns resource type + fields" },
    { id: "configure_rpc", class: "box", x: 780, y: 390, label: "ConfigureProvider RPC\nprovider config: credentials, endpoint" },
    { id: "client", class: "box", x: 1120, y: 390, label: "Provider API client\nreturned as ConfigureResponse.ResourceData" },
    { id: "plan_apply", class: "box", x: 390, y: 570, label: "PlanResourceChange → ApplyResourceChange\nModifyPlan; then Create, Update or Delete" },
    { id: "remote", class: "box", x: 780, y: 570, label: "Remote service API\nresource operation; response becomes state" },
  ],
  edges: [
    { from: "config", to: "core", label: "provider address + resource configuration" },
    { from: "core", to: "binary", label: "discover and start selected provider executable" },
    { from: "binary", to: "server", label: "main() calls providerserver.Serve" },
    { from: "server", to: "core", label: "handshake advertises protocol v6 + gRPC address" },
    { from: "core", to: "server", label: "Protocol Buffers over gRPC" },
    { from: "server", to: "framework", label: "register tfplugin6.ProviderServer endpoints" },
    { from: "framework", to: "schema_rpc", label: "dispatch GetProviderSchema" },
    { from: "schema_rpc", to: "provider", label: "invoke provider.Schema()" },
    { from: "provider", to: "resource_factory", label: "Resources() enumerates factories" },
    { from: "resource_factory", to: "resource", label: "instantiate resource; call its Schema()" },
    { from: "resource", to: "framework", label: "provider-defined schema returned through RPC" },
    { from: "framework", to: "schema_result", label: "return schema map to Terraform Core" },
    { from: "core", to: "configure_rpc", label: "plan/apply: ConfigureProvider(provider config)" },
    { from: "framework", to: "provider", label: "dispatch to provider.Configure()" },
    { from: "provider", to: "client", label: "ResourceData carries configured API client" },
    { from: "core", to: "plan_apply", label: "plan then apply resource config + prior state" },
    { from: "framework", to: "plan_apply", label: "PlanResourceChange / ApplyResourceChange dispatch" },
    { from: "plan_apply", to: "resource", label: "call ModifyPlan; choose Create, Update or Delete" },
    { from: "resource", to: "client", label: "configured client passed to resource Configure()" },
    { from: "client", to: "remote", label: "provider implementation makes API request" },
    { from: "remote", to: "resource", label: "API result / diagnostics" },
    { from: "resource", to: "framework", label: "new resource state + diagnostics" },
    { from: "framework", to: "core", label: "ApplyResourceChange response over gRPC" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
