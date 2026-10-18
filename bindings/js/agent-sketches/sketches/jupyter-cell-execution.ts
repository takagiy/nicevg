// Jupyter — https://docs.jupyter.org/en/latest/projects/architecture/content-architecture.html
// https://jupyter-server.readthedocs.io/en/latest/developers/websocket-protocols.html
// https://jupyter-protocol.readthedocs.io/en/latest/messaging.html
// https://ipython.readthedocs.io/en/stable/development/execution.html
// https://github.com/ipython/ipykernel/blob/main/ipykernel/kernelbase.py
// A user runs one notebook code cell. The sketch follows execute_request through the browser,
// Jupyter Server and IPython kernel, then shows output, errors, status and execute_reply returning.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
  fill?: string;
  width?: number;
};

const sketch = {
  nodes: [
    { id: "browser", class: "box", x: 20, y: 20, width: 250, label: "JupyterLab browser", fill: "#eff6ff" },
    { id: "cell", class: "box", x: 45, y: 70, parent: "browser", label: "Code cell\nuser clicks Run" },
    { id: "kernel-client", class: "box", x: 45, y: 175, parent: "browser", label: "@jupyterlab/services\nKernel.requestExecute" },
    { id: "output-area", class: "box", x: 45, y: 295, parent: "browser", label: "Cell output area\nrender stream / result / error" },

    { id: "server", class: "box", x: 330, y: 20, width: 320, label: "Jupyter Server", fill: "#f0fdf4" },
    { id: "websocket", class: "box", x: 365, y: 85, parent: "server", label: "Kernel WebSocket\nshell + iopub multiplexed" },
    { id: "kernel-manager", class: "box", x: 365, y: 205, parent: "server", label: "MappingKernelManager\ntracks kernel id" },

    { id: "kernel", class: "box", x: 710, y: 20, width: 350, label: "IPython kernel process", fill: "#fff7ed" },
    { id: "shell", class: "box", x: 745, y: 85, parent: "kernel", label: "Shell ROUTER socket\nexecute_request" },
    { id: "execute-handler", class: "box", x: 745, y: 195, parent: "kernel", label: "KernelBase.do_execute\nstatus busy; execute_input" },
    { id: "run-cell", class: "box", x: 745, y: 315, parent: "kernel", label: "IPython run_cell\ntransform, compile, execute" },
    { id: "namespace", class: "box", x: 745, y: 425, parent: "kernel", label: "Interactive user namespace" },
    { id: "iopub", class: "box", x: 880, y: 425, parent: "kernel", label: "IOPub PUB socket\nstream / result / error; idle" },
    { id: "reply", class: "box", x: 745, y: 535, parent: "kernel", label: "Shell execute_reply\nstatus + execution_count" },
  ] satisfies Node[],
  edges: [
    { from: "cell", to: "kernel-client", label: "cell code" },
    { from: "kernel-client", to: "websocket", label: "execute_request\ncode, silent, store_history" },
    { from: "websocket", to: "kernel-manager", label: "kernel id + shell channel" },
    { from: "kernel-manager", to: "shell", label: "route request to kernel" },
    { from: "shell", to: "execute-handler", label: "execute_request" },
    { from: "execute-handler", to: "run-cell", label: "code" },
    { from: "run-cell", to: "namespace", label: "evaluate in shared state" },
    { from: "namespace", to: "run-cell", label: "value or exception" },
    { from: "run-cell", to: "iopub", label: "display_data / execute_result\nor error traceback" },
    { from: "execute-handler", to: "reply", label: "ok or error; execution_count" },
    { from: "iopub", to: "websocket", label: "IOPub status + output\nparent_header links request" },
    { from: "websocket", to: "kernel-client", label: "IOPub messages" },
    { from: "kernel-client", to: "output-area", label: "render each output" },
    { from: "reply", to: "websocket", label: "shell execute_reply" },
    { from: "websocket", to: "kernel-client", label: "shell reply; future completes\nafter reply and idle" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
