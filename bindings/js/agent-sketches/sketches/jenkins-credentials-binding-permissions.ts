// Jenkins Credentials Binding, based on the plugin guide, step reference, core extension index,
// and credentials permission documentation. This follows one Pipeline credential from its
// scoped lookup into an environment binding and the ConsoleLogFilter extension that masks logs.
// Sources: https://www.jenkins.io/doc/pipeline/steps/credentials-binding/
//          https://www.jenkins.io/doc/developer/extensions/jenkins-core/
//          https://www.jenkins.io/doc/book/security/access-control/permissions/
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string };

const sketch = {
  nodes: [
    { id: "plugin", class: "box", x: 30, y: 30, label: "credentials-binding.hpi\nplugin archive" },
    { id: "extension", class: "box", x: 320, y: 30, label: "@Extension\nBindingStep.DescriptorImpl" },
    { id: "step", class: "box", x: 630, y: 30, label: "withCredentials\nstep descriptor / functionName" },
    { id: "script", class: "box", x: 950, y: 30, label: "Pipeline script\ncredentialsId, variable" },
    { id: "bindings", class: "box", x: 950, y: 190, label: "MultiBinding[]\ncredentialsId, variable(s)" },
    { id: "context", class: "box", x: 630, y: 190, label: "StepContext\nRun, TaskListener, EnvVars" },
    { id: "run", class: "box", x: 320, y: 190, label: "Run / Job\nItem context for lookup" },
    { id: "auth", class: "box", x: 30, y: 190, label: "Authentication\ncurrent build identity" },
    { id: "lookup", class: "box", x: 30, y: 360, label: "CredentialsProvider\nlookupCredentialsInItem" },
    { id: "permission", class: "box", x: 320, y: 360, label: "Credentials / Item permissions\nView, UseItem, Job/Configure" },
    { id: "store", class: "box", x: 630, y: 360, label: "CredentialsStore\ncontextual credential domain" },
    { id: "credential", class: "box", x: 950, y: 360, label: "StringCredentials\nid, secret" },
    { id: "binding", class: "box", x: 950, y: 530, label: "StringBinding\nsecret -> environment variable" },
    { id: "env", class: "box", x: 630, y: 530, label: "EnvironmentExpander\nwith secret variable override" },
    { id: "body", class: "box", x: 320, y: 530, label: "BodyInvoker\nstart body with merged context" },
    { id: "filter", class: "box", x: 30, y: 530, label: "SecretBuildWrapper.Filter\nConsoleLogFilter extension" },
    { id: "patterns", class: "box", x: 30, y: 700, label: "SecretPatterns\nraw + shell-mangled forms" },
    { id: "logger", class: "box", x: 320, y: 700, label: "TaskListener / logger\nbuild output stream" },
    { id: "mask", class: "box", x: 630, y: 700, label: "MaskingOutputStream\nsecret bytes -> ****" },
    { id: "agent", class: "box", x: 950, y: 700, label: "Build step on agent\nsh / bat consumes env" },
    { id: "unbind", class: "box", x: 950, y: 870, label: "Unbinder[]\nremove env / temporary files" },
    { id: "cleanup", class: "box", x: 630, y: 870, label: "Body completion\nclose filter; unbind" },
    { id: "log", class: "box", x: 320, y: 870, label: "Build log\nmasked output" },
    { id: "config", class: "box", x: 30, y: 870, label: "Job configuration\ncredential ID reference only" },
  ],
  edges: [
    { from: "plugin", to: "extension" }, { from: "extension", to: "step" },
    { from: "step", to: "script" }, { from: "script", to: "bindings" },
    { from: "script", to: "context" }, { from: "context", to: "run" },
    { from: "context", to: "auth" }, { from: "bindings", to: "lookup" },
    { from: "run", to: "lookup" }, { from: "auth", to: "lookup" },
    { from: "permission", to: "lookup" }, { from: "lookup", to: "store" },
    { from: "store", to: "credential" }, { from: "credential", to: "bindings" },
    { from: "bindings", to: "binding" }, { from: "binding", to: "env" },
    { from: "binding", to: "patterns" }, { from: "binding", to: "unbind" },
    { from: "env", to: "body" }, { from: "body", to: "agent" },
    { from: "body", to: "filter" }, { from: "patterns", to: "filter" },
    { from: "filter", to: "logger" }, { from: "logger", to: "mask" },
    { from: "mask", to: "log" }, { from: "agent", to: "logger" },
    { from: "agent", to: "unbind" }, { from: "unbind", to: "cleanup" },
    { from: "cleanup", to: "body" }, { from: "config", to: "script" },
    { from: "permission", to: "config" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
