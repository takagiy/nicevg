// Neovim remote-plugin hooks, following the remote-plugin and API documentation.
// This traces the documented Python `Limit` plugin from discovery and manifest generation
// through lazy host startup, command/autocmd/function dispatch, and synchronous RPC replies.
// Sources: https://neovim.io/doc/user/remote_plugin/
//          https://neovim.io/doc/user/api/
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
    { id: "core", class: "box", x: 30, y: 30, label: "Neovim process\ncore + RPC server" },
    { id: "runtime", class: "box", x: 30, y: 180, label: "'runtimepath' directory\ncontains rplugin/python/limit.py" },
    { id: "update", class: "box", x: 30, y: 340, label: ":UpdateRemotePlugins\nexplicit after install / update / delete" },
    { id: "scanner", class: "box", x: 360, y: 340, label: "remote host update\nscans rplugin/{host} on runtime paths" },
    { id: "import_scan", class: "box", x: 700, y: 340, label: "pynvim discovery\nreads decorators; avoids plugin side effects" },
    { id: "manifest", class: "box", x: 1060, y: 340, label: "$XDG_DATA_HOME/nvim/rplugin.vim\n(or $NVIM_RPLUGIN_MANIFEST)" },
    { id: "register", class: "box", x: 1060, y: 180, label: "remote#host#RegisterPlugin\npython3 + path + exported specs" },
    { id: "load_manifest", class: "box", x: 700, y: 180, label: "source manifest at startup\nregister stubs; do not spawn host" },
    { id: "host_config", class: "box", x: 360, y: 30, label: "Python provider config\ng:python3_host_prog / pynvim available" },
    { id: "first_use", class: "box", x: 700, y: 30, label: "first matching hook used\nbootstrap host lazily" },
    { id: "python_host", class: "box", x: 1060, y: 30, label: "Python host process\nseparate coprocess over Msgpack-RPC" },
    { id: "plugin_class", class: "box", x: 1420, y: 30, label: "Limit class\n@pynvim.plugin\n__init__(vim); calls = 0" },
    { id: "command_decl", class: "box", x: 1420, y: 180, label: "@pynvim.command('Cmd'...)\nargs + range; sync=True" },
    { id: "autocmd_decl", class: "box", x: 1420, y: 340, label: "@pynvim.autocmd('BufEnter')\npattern='*.py'; eval=<afile>; sync=True" },
    { id: "function_decl", class: "box", x: 1420, y: 500, label: "@pynvim.function('Func')\nargs; sync default" },
    { id: "cmd_stub", class: "box", x: 1060, y: 500, label: "manifest command stub\nCmd; forwards args + range" },
    { id: "auto_stub", class: "box", x: 700, y: 500, label: "manifest autocmd stub\nBufEnter *.py; evaluates <afile>" },
    { id: "func_stub", class: "box", x: 360, y: 500, label: "manifest function stub\nFunc(args)" },
    { id: "cmd_trigger", class: "box", x: 30, y: 500, label: ":Cmd [args]\nEx command from editor/user" },
    { id: "buf_trigger", class: "box", x: 30, y: 660, label: "BufEnter on *.py\nNvim event + matched filename" },
    { id: "func_trigger", class: "box", x: 360, y: 660, label: "call Func(args)\nVimscript / API caller" },
    { id: "rpc_request", class: "box", x: 700, y: 660, label: "rpcrequest(host, method, args)\nsync hooks block for result" },
    { id: "rpc_notify", class: "box", x: 1060, y: 660, label: "rpcnotify(host, method, args)\nasync hooks: fire-and-forget" },
    { id: "handlers", class: "box", x: 1420, y: 660, label: "command_handler / autocmd_handler\nfunction_handler(args)" },
    { id: "counter", class: "box", x: 1780, y: 660, label: "_increment_calls()\nif calls == 5: raise Exception\nelse calls += 1" },
    { id: "buffer_line", class: "box", x: 1780, y: 500, label: "vim.current.line = message\nCommand / Autocmd / Function result" },
    { id: "api_set", class: "box", x: 1780, y: 340, label: "Neovim API over RPC\ncurrent buffer line assignment" },
    { id: "return_value", class: "box", x: 1060, y: 820, label: "handler return value\nreturned to rpcrequest caller" },
    { id: "exception", class: "box", x: 1420, y: 820, label: "handler exception\nsync call reports error to Nvim" },
    { id: "async_drop", class: "box", x: 700, y: 820, label: "async handler result / exception\nignored by rpcnotify caller" },
  ] satisfies Node[],
  edges: [
    { from: "runtime", to: "update", label: "plugin file installed on runtime path" },
    { from: "core", to: "load_manifest", label: "startup sources generated manifest" },
    { from: "update", to: "scanner", label: "invoke manifest generation" },
    { from: "scanner", to: "import_scan", label: "discover host plugin source" },
    { from: "import_scan", to: "manifest", label: "write Vimscript declarations" },
    { from: "manifest", to: "register", label: "contains RegisterPlugin('python3', path, specs)" },
    { from: "register", to: "load_manifest", label: "sourced during Nvim startup" },
    { from: "load_manifest", to: "cmd_stub", label: "declare command forwarding stub" },
    { from: "load_manifest", to: "auto_stub", label: "declare autocmd forwarding stub" },
    { from: "load_manifest", to: "func_stub", label: "declare function forwarding stub" },
    { from: "host_config", to: "python_host", label: "select Python executable; require pynvim" },
    { from: "cmd_trigger", to: "cmd_stub", label: "execute exported :Cmd" },
    { from: "buf_trigger", to: "auto_stub", label: "event matches BufEnter + *.py" },
    { from: "func_trigger", to: "func_stub", label: "invoke exported Func(args)" },
    { from: "cmd_stub", to: "first_use", label: "first matching invocation" },
    { from: "auto_stub", to: "first_use", label: "first matching invocation" },
    { from: "func_stub", to: "first_use", label: "first matching invocation" },
    { from: "first_use", to: "python_host", label: "start host once on demand" },
    { from: "python_host", to: "plugin_class", label: "load plugin path; instantiate Limit(vim)" },
    { from: "plugin_class", to: "command_decl", label: "decorator metadata exported" },
    { from: "plugin_class", to: "autocmd_decl", label: "decorator metadata exported" },
    { from: "plugin_class", to: "function_decl", label: "decorator metadata exported" },
    { from: "command_decl", to: "cmd_stub", label: "register Cmd with nargs + range" },
    { from: "autocmd_decl", to: "auto_stub", label: "register BufEnter pattern + eval" },
    { from: "function_decl", to: "func_stub", label: "register Func(args)" },
    { from: "cmd_stub", to: "rpc_request", label: "sync=True: rpcrequest" },
    { from: "auto_stub", to: "rpc_request", label: "sync=True: rpcrequest" },
    { from: "func_stub", to: "rpc_request", label: "function return requires request/reply" },
    { from: "rpc_request", to: "handlers", label: "Msgpack-RPC method + arguments" },
    { from: "python_host", to: "rpc_notify", label: "used for declarations without sync=True" },
    { from: "rpc_notify", to: "handlers", label: "async dispatch; no result awaited" },
    { from: "handlers", to: "counter", label: "each handler calls _increment_calls()" },
    { from: "counter", to: "exception", label: "when calls already equals 5" },
    { from: "counter", to: "buffer_line", label: "otherwise increment and format message" },
    { from: "handlers", to: "api_set", label: "write output through vim proxy" },
    { from: "api_set", to: "core", label: "Neovim API request updates current line" },
    { from: "handlers", to: "return_value", label: "synchronous function result" },
    { from: "return_value", to: "rpc_request", label: "RPC response unblocks caller" },
    { from: "exception", to: "rpc_request", label: "RPC error response unblocks with error" },
    { from: "async_drop", to: "rpc_notify", label: "return and raised exception are ignored" },
    { from: "rpc_notify", to: "async_drop", label: "no response channel" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
