import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// Sources:
// https://docs.github.com/en/actions/how-tos/manage-runners/use-actions-runner-controller/deploy-runner-scale-sets
// https://github.com/actions/actions-runner-controller/blob/master/docs/gha-runner-scale-set-controller/README.md
// https://github.com/actions/actions-runner-controller/blob/master/charts/gha-runner-scale-set-controller/values.yaml
// https://github.com/actions/actions-runner-controller/blob/master/charts/gha-runner-scale-set/values.yaml
// This sketch explains how a runner scale set is deployed across separate ARC and runner namespaces:
// the listener acquires workflow jobs, ARC creates ephemeral runner pods, and the runner reports job completion.

const nodes: Node[] = [
  { id: "developer", class: "box", x: 20, y: 70, label: "Workflow author\n.github/workflows/*.yml", fill: "#f1f5f9" },
  { id: "workflow", class: "box", x: 20, y: 180, label: "Workflow job\nruns-on: arc-linux", fill: "#fef3c7" },
  { id: "github", class: "box", x: 20, y: 330, label: "GitHub repository / org\nActions scheduler", parent: "github-cloud", fill: "#f1f5f9" },
  { id: "actions-service", class: "box", x: 260, y: 330, label: "Actions Service\nscale-set registration · job queue", parent: "github-cloud", fill: "#fee2e2" },
  { id: "github-cloud", class: "box", x: 0, y: 285, label: "GitHub hosted services", width: 500, height: 240, fill: "#fff1f2" },
  { id: "cluster", class: "box", x: 550, y: 20, label: "Kubernetes cluster", width: 1110, height: 960, fill: "#eff6ff" },
  { id: "api", class: "box", x: 590, y: 70, label: "Kubernetes API server", parent: "cluster", fill: "#dbeafe" },
  { id: "operator-ns", class: "box", x: 620, y: 150, label: "arc-systems namespace", width: 470, height: 340, parent: "cluster", fill: "#f8fafc" },
  { id: "controller-deploy", class: "box", x: 650, y: 200, label: "gha-runner-scale-set-controller\nDeployment", parent: "operator-ns", fill: "#dcfce7" },
  { id: "controller-manager", class: "box", x: 650, y: 300, label: "controller-manager container", parent: "controller-deploy" },
  { id: "autoscaling-controller", class: "box", x: 880, y: 200, label: "AutoScalingRunnerSet\ncontroller", parent: "operator-ns" },
  { id: "runner-controller", class: "box", x: 880, y: 300, label: "EphemeralRunner\ncontroller", parent: "operator-ns" },
  { id: "listener-controller", class: "box", x: 880, y: 400, label: "AutoScalingListener\ncontroller", parent: "operator-ns" },
  { id: "secret", class: "box", x: 650, y: 410, label: "Kubernetes Secret\nGitHub App or PAT credentials", parent: "operator-ns", fill: "#fef3c7" },
  { id: "runner-ns", class: "box", x: 1130, y: 150, label: "arc-runners namespace", width: 490, height: 700, parent: "cluster", fill: "#f0fdf4" },
  { id: "autoscaling-resource", class: "box", x: 1160, y: 200, label: "AutoScalingRunnerSet\narc-linux · minRunners · maxRunners", parent: "runner-ns", fill: "#fef3c7" },
  { id: "listener-resource", class: "box", x: 1160, y: 300, label: "RunnerScaleSetListener\ncustom resource", parent: "runner-ns" },
  { id: "listener-pod", class: "box", x: 1390, y: 300, label: "Listener Pod\nlistener container", parent: "runner-ns", fill: "#dbeafe" },
  { id: "listener-sa", class: "box", x: 1160, y: 400, label: "Listener ServiceAccount\n+ Role / RoleBinding", parent: "runner-ns" },
  { id: "ers", class: "box", x: 1160, y: 500, label: "EphemeralRunnerSet\ndesired replica count", parent: "runner-ns", fill: "#fef3c7" },
  { id: "er-controller", class: "box", x: 1390, y: 500, label: "EphemeralRunner\ncontroller reconciliation", parent: "runner-ns" },
  { id: "jit", class: "box", x: 1160, y: 600, label: "JIT configuration token\nper runner registration", parent: "runner-ns" },
  { id: "runner-pod", class: "box", x: 1390, y: 610, label: "Ephemeral runner Pod\nrunner container · one job", parent: "runner-ns", fill: "#dcfce7" },
  { id: "runner", class: "box", x: 1390, y: 700, label: "GitHub Actions Runner\nconnects outbound to Actions", parent: "runner-pod" },
  { id: "container-job", class: "box", x: 1160, y: 740, label: "Container job / service / action Pod\noptional Kubernetes container mode", parent: "runner-ns", fill: "#e0f2fe" },
  { id: "job-storage", class: "box", x: 1160, y: 820, label: "Optional RWX volume or\nlifecycle-hook file transfer", parent: "runner-ns" },
  { id: "node-pool", class: "box", x: 850, y: 710, label: "Kubernetes worker nodes\nScheduler places runner pods", parent: "cluster", fill: "#f1f5f9" },
  { id: "registry", class: "box", x: 20, y: 600, label: "Container registry\nrunner image pull", fill: "#f1f5f9" },
  { id: "metrics", class: "box", x: 650, y: 540, label: "Controller / listener metrics\nPrometheus scrape endpoints", parent: "cluster" },
  { id: "prometheus", class: "box", x: 20, y: 740, label: "Prometheus", fill: "#f1f5f9" },
  { id: "logs", class: "box", x: 20, y: 850, label: "Cluster log collection\ncontroller · listener · runner", fill: "#f1f5f9" },
  { id: "failed", class: "box", x: 880, y: 600, label: "Failed runner pod\nretry creation up to 5 times", parent: "cluster", fill: "#fee2e2" },
];

const edges = [
  { from: "developer", to: "workflow", label: "commits workflow using runs-on" },
  { from: "workflow", to: "github", label: "triggers job" },
  { from: "github", to: "actions-service", label: "route matching runner scale set job" },
  { from: "autoscaling-resource", to: "autoscaling-controller", label: "watch desired config" },
  { from: "autoscaling-controller", to: "secret", label: "read GitHub credentials" },
  { from: "autoscaling-controller", to: "actions-service", label: "HTTPS API: resolve runner group; get/create scale set" },
  { from: "autoscaling-controller", to: "listener-resource", label: "create listener resource" },
  { from: "listener-resource", to: "listener-controller", label: "watch resource" },
  { from: "listener-controller", to: "listener-pod", label: "create listener Pod" },
  { from: "listener-pod", to: "secret", label: "GitHub App installation token" },
  { from: "listener-pod", to: "actions-service", label: "HTTPS long poll; Job Available" },
  { from: "actions-service", to: "listener-pod", label: "job message; acknowledge when capacity allows" },
  { from: "listener-pod", to: "listener-sa", label: "use Kubernetes identity" },
  { from: "listener-sa", to: "api", label: "RBAC-authorized HTTPS patch" },
  { from: "listener-pod", to: "ers", label: "patch desired replicas from assigned jobs + minRunners" },
  { from: "ers", to: "er-controller", label: "watch replica target" },
  { from: "er-controller", to: "actions-service", label: "request JIT runner configuration" },
  { from: "actions-service", to: "jit", label: "short-lived JIT config" },
  { from: "er-controller", to: "runner-pod", label: "create one ephemeral Pod per runner" },
  { from: "jit", to: "runner", label: "configure and register ephemeral runner" },
  { from: "runner-pod", to: "node-pool", label: "scheduled onto worker node" },
  { from: "runner-pod", to: "registry", label: "pull configured runner image" },
  { from: "runner", to: "actions-service", label: "outbound HTTPS: accept job, stream logs, status" },
  { from: "actions-service", to: "runner", label: "workflow job payload / step instructions" },
  { from: "runner", to: "container-job", label: "create job, service or container-action pod (optional)" },
  { from: "runner", to: "job-storage", label: "share job filesystem (configured mode)" },
  { from: "container-job", to: "job-storage", label: "mount shared volume when configured" },
  { from: "runner", to: "ers", label: "job finishes; ephemeral runner removed" },
  { from: "er-controller", to: "failed", label: "failed Pod status" },
  { from: "failed", to: "er-controller", label: "retry create, max 5 attempts" },
  { from: "controller-manager", to: "metrics", label: "expose controller metrics" },
  { from: "listener-pod", to: "metrics", label: "expose listener job / runner metrics" },
  { from: "prometheus", to: "metrics", label: "scrape endpoints" },
  { from: "logs", to: "controller-manager", label: "collect container logs" },
  { from: "logs", to: "listener-pod", label: "collect container logs" },
  { from: "logs", to: "runner-pod", label: "retain ephemeral runner logs" },
  { from: "api", to: "controller-manager", label: "watch custom resources; reconcile" },
  { from: "api", to: "er-controller", label: "create / observe runner Pods" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
