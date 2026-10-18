import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// Kubernetes kubeadm HA stacked-etcd deployment, based on:
// https://kubernetes.io/docs/setup/production-environment/tools/kubeadm/high-availability/
// https://kubernetes.io/docs/setup/production-environment/tools/kubeadm/ha-topology/
// https://kubernetes.io/docs/concepts/architecture/
// This shows the reference deployment shape: a TCP load balancer fronts three control-plane
// hosts, each with a static-pod control plane and local etcd member; worker nodes run the node agents.

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; fill?: string; width?: number; height?: number };
const nodes: Node[] = [
  { id: "operator", class: "box", x: 20, y: 170, label: "Cluster operator\nkubectl", fill: "#fef3c7" },
  { id: "dns", class: "box", x: 20, y: 290, label: "DNS name\nControlPlaneEndpoint", fill: "#fef3c7" },
  { id: "lb", class: "box", x: 240, y: 200, label: "TCP load balancer\n:6443 health checks", fill: "#ffedd5" },
  { id: "registry", class: "box", x: 240, y: 570, label: "registry.k8s.io\ncontrol-plane images", fill: "#fef3c7" },
  { id: "network", class: "box", x: 240, y: 710, label: "Chosen CNI plugin\nPod IPs + pod routing", fill: "#fef3c7" },
  { id: "cp-boundary", class: "box", x: 520, y: 30, label: "Dedicated control-plane machines · kubeadm · static Pods", fill: "#eff6ff", width: 980, height: 420 },
  { id: "cp1", class: "box", x: 550, y: 80, label: "Control-plane node 1", parent: "cp-boundary", fill: "#dbeafe", width: 270, height: 320 },
  { id: "api1", class: "box", x: 575, y: 120, label: "kube-apiserver", parent: "cp1" },
  { id: "sched1", class: "box", x: 575, y: 185, label: "kube-scheduler", parent: "cp1" },
  { id: "cm1", class: "box", x: 575, y: 250, label: "kube-controller-manager", parent: "cp1" },
  { id: "etcd1", class: "box", x: 575, y: 315, label: "etcd member 1\n/var/lib/etcd", parent: "cp1" },
  { id: "cp2", class: "box", x: 860, y: 80, label: "Control-plane node 2", parent: "cp-boundary", fill: "#dbeafe", width: 270, height: 320 },
  { id: "api2", class: "box", x: 885, y: 120, label: "kube-apiserver", parent: "cp2" },
  { id: "sched2", class: "box", x: 885, y: 185, label: "kube-scheduler", parent: "cp2" },
  { id: "cm2", class: "box", x: 885, y: 250, label: "kube-controller-manager", parent: "cp2" },
  { id: "etcd2", class: "box", x: 885, y: 315, label: "etcd member 2\n/var/lib/etcd", parent: "cp2" },
  { id: "cp3", class: "box", x: 1170, y: 80, label: "Control-plane node 3", parent: "cp-boundary", fill: "#dbeafe", width: 270, height: 320 },
  { id: "api3", class: "box", x: 1195, y: 120, label: "kube-apiserver", parent: "cp3" },
  { id: "sched3", class: "box", x: 1195, y: 185, label: "kube-scheduler", parent: "cp3" },
  { id: "cm3", class: "box", x: 1195, y: 250, label: "kube-controller-manager", parent: "cp3" },
  { id: "etcd3", class: "box", x: 1195, y: 315, label: "etcd member 3\n/var/lib/etcd", parent: "cp3" },
  { id: "workers", class: "box", x: 520, y: 500, label: "Worker machines · kubeadm join", fill: "#f0fdf4", width: 980, height: 330 },
  { id: "w1", class: "box", x: 550, y: 550, label: "Worker node 1", parent: "workers", fill: "#dcfce7", width: 270, height: 230 },
  { id: "kl1", class: "box", x: 570, y: 590, label: "kubelet", parent: "w1" },
  { id: "rt1", class: "box", x: 570, y: 650, label: "CRI runtime\n(containerd / CRI-O)", parent: "w1" },
  { id: "proxy1", class: "box", x: 570, y: 715, label: "kube-proxy", parent: "w1" },
  { id: "pods1", class: "box", x: 700, y: 650, label: "Scheduled workload Pods", parent: "w1" },
  { id: "w2", class: "box", x: 860, y: 550, label: "Worker node 2", parent: "workers", fill: "#dcfce7", width: 270, height: 230 },
  { id: "kl2", class: "box", x: 880, y: 590, label: "kubelet", parent: "w2" },
  { id: "rt2", class: "box", x: 880, y: 650, label: "CRI runtime\n(containerd / CRI-O)", parent: "w2" },
  { id: "proxy2", class: "box", x: 880, y: 715, label: "kube-proxy", parent: "w2" },
  { id: "pods2", class: "box", x: 1010, y: 650, label: "Scheduled workload Pods", parent: "w2" },
  { id: "w3", class: "box", x: 1170, y: 550, label: "Worker node 3", parent: "workers", fill: "#dcfce7", width: 270, height: 230 },
  { id: "kl3", class: "box", x: 1190, y: 590, label: "kubelet", parent: "w3" },
  { id: "rt3", class: "box", x: 1190, y: 650, label: "CRI runtime\n(containerd / CRI-O)", parent: "w3" },
  { id: "proxy3", class: "box", x: 1190, y: 715, label: "kube-proxy", parent: "w3" },
  { id: "pods3", class: "box", x: 1320, y: 650, label: "Scheduled workload Pods", parent: "w3" },
];

const edges = [
  { from: "operator", to: "dns", label: "uses endpoint" },
  { from: "dns", to: "lb", label: "resolves to" },
  { from: "operator", to: "lb", label: "HTTPS API :6443" },
  { from: "lb", to: "api1", label: "TCP :6443; healthy backends" },
  { from: "lb", to: "api2", label: ":6443" },
  { from: "lb", to: "api3", label: ":6443" },
  { from: "api1", to: "etcd1", label: "API data" },
  { from: "api2", to: "etcd2", label: "API data" },
  { from: "api3", to: "etcd3", label: "API data" },
  { from: "etcd1", to: "etcd2", label: "peer :2380" },
  { from: "etcd2", to: "etcd3", label: "peer :2380" },
  { from: "etcd3", to: "etcd1", label: "peer :2380" },
  { from: "sched1", to: "api1", label: "watch Pods; bind node" },
  { from: "sched2", to: "api2", label: "watch Pods; bind node" },
  { from: "sched3", to: "api3", label: "watch Pods; bind node" },
  { from: "cm1", to: "api1", label: "watch + reconcile APIs" },
  { from: "cm2", to: "api2", label: "watch + reconcile APIs" },
  { from: "cm3", to: "api3", label: "watch + reconcile APIs" },
  { from: "registry", to: "api1", label: "image pull during bootstrap" },
  { from: "registry", to: "rt1", label: "container images" },
  { from: "registry", to: "rt2", label: "container images" },
  { from: "registry", to: "rt3", label: "container images" },
  { from: "network", to: "pods1", label: "CNI ADD: IP + routes" },
  { from: "network", to: "pods2", label: "CNI ADD: IP + routes" },
  { from: "network", to: "pods3", label: "CNI ADD: IP + routes" },
  { from: "kl1", to: "lb", label: "API watch / status via endpoint" },
  { from: "kl2", to: "lb", label: "API watch / status via endpoint" },
  { from: "kl3", to: "lb", label: "API watch / status via endpoint" },
  { from: "kl1", to: "rt1", label: "CRI: create Pod sandbox / containers" },
  { from: "kl2", to: "rt2", label: "CRI: create Pod sandbox / containers" },
  { from: "kl3", to: "rt3", label: "CRI: create Pod sandbox / containers" },
  { from: "rt1", to: "pods1", label: "runs containers" },
  { from: "rt2", to: "pods2", label: "runs containers" },
  { from: "rt3", to: "pods3", label: "runs containers" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
