// PyTorch DataLoader: https://pytorch.org/docs/stable/data.html
// Implementation: https://github.com/pytorch/pytorch/blob/main/torch/utils/data/dataloader.py
// Worker loop: https://github.com/pytorch/pytorch/blob/main/torch/utils/data/_utils/worker.py
// A map-style batch travels from the main-process sampler through worker fetch/collation and
// optional pinning. The side paths show how worker exceptions and stalled workers reach the iterator.

import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type N = SketchNode & { class: "module" | "queue" | "boundary"; label: string; fill?: string };
const module = ({ label, fill = "#eff6ff" }: N) => {
  const t = text(label, 12), w = t.width + 26, h = t.height + 18;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="9" fill="${fill}" stroke="#334155"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const queue = ({ label }: N) => {
  const t = text(label, 11), w = t.width + 24, h = t.height + 18;
  return svg`<path data-bounds="true" d="M 0 5 Q ${w / 2} -2 ${w} 5 L ${w} ${h - 5} Q ${w / 2} ${h + 2} 0 ${h - 5} Z" fill="#fef3c7" stroke="#92400e"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const boundary = ({ label }: N) => {
  const t = text(label, 12), w = t.width + 30, h = t.height + 18;
  return svg`<path data-bounds="true" d="M 14 0 H ${w} V ${h} H 0 V 14 Z" fill="#f8fafc" stroke="#64748b" stroke-dasharray="5 3"/>${t.at({ left: 24, cy: h / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "consumer", class: "module", x: 35, y: 190, label: "training loop\nnext(loader)" },
    { id: "loader", class: "module", x: 220, y: 185, label: "DataLoader\nconstruct iterator" },
    { id: "iterator", class: "module", x: 410, y: 190, label: "_MultiProcessing\nDataLoaderIter" },
    { id: "sampler", class: "module", x: 395, y: 55, label: "_sampler_iter\nSampler indices" },
    { id: "indexq", class: "queue", x: 615, y: 70, label: "index_queue\n(worker_id, index)" },
    { id: "worker", class: "boundary", x: 800, y: 20, label: "worker subprocess" },
    { id: "workerloop", class: "module", parent: "worker", x: 820, y: 67, label: "_worker_loop" },
    { id: "fetcher", class: "module", parent: "worker", x: 1015, y: 67, label: "_MapDatasetFetcher" },
    { id: "getitem", class: "module", parent: "worker", x: 1000, y: 160, label: "Dataset.__getitem__\nread sample" },
    { id: "transform", class: "module", parent: "worker", x: 1190, y: 170, label: "sample transforms" },
    { id: "collate", class: "module", parent: "worker", x: 1200, y: 65, label: "collate_fn\n(default_collate)" },
    { id: "resultq", class: "queue", x: 1400, y: 90, label: "worker_result_queue\n(data or ExceptionWrapper)" },
    { id: "pin", class: "module", x: 1415, y: 215, label: "_pin_memory_loop\n(optional)" },
    { id: "dataq", class: "queue", x: 1605, y: 220, label: "data_queue" },
    { id: "getdata", class: "module", x: 1600, y: 105, label: "_get_data()\nwait / timeout" },
    { id: "process", class: "module", x: 1810, y: 155, label: "_process_data()\nreturn batch" },
    { id: "output", class: "module", x: 2005, y: 185, label: "batch to training loop", fill: "#dcfce7" },
    { id: "workererror", class: "module", x: 1010, y: 320, label: "ExceptionWrapper\nworker traceback", fill: "#fee2e2" },
    { id: "reraise", class: "module", x: 1400, y: 365, label: "reraise()\noriginal exception", fill: "#fee2e2" },
    { id: "timeout", class: "module", x: 1610, y: 365, label: "DataLoader timeout\nraises RuntimeError", fill: "#fee2e2" },
    { id: "shutdown", class: "module", x: 815, y: 435, label: "workers_done_event\nshutdown signal" },
    { id: "workerexception", class: "module", x: 1000, y: 435, label: "worker catches Exception\nwraps traceback" },
    { id: "single", class: "module", x: 400, y: 360, label: "num_workers = 0\n_fetcher runs inline" },
  ] satisfies N[],
  edges: [
    { from: "consumer", to: "loader" },
    { from: "loader", to: "iterator" },
    { from: "iterator", to: "sampler", label: "next index" },
    { from: "sampler", to: "indexq", label: "prefetch indices" },
    { from: "indexq", to: "workerloop" },
    { from: "workerloop", to: "fetcher" },
    { from: "fetcher", to: "getitem" },
    { from: "getitem", to: "transform" },
    { from: "transform", to: "collate" },
    { from: "collate", to: "resultq", label: "batch" },
    { from: "resultq", to: "pin", label: "when enabled" },
    { from: "pin", to: "dataq", label: "pinned tensors" },
    { from: "resultq", to: "getdata", label: "pinning disabled" },
    { from: "dataq", to: "getdata" },
    { from: "getdata", to: "process" },
    { from: "process", to: "output" },
    { from: "getitem", to: "workerexception", label: "fetch / transform / collate raises" },
    { from: "workerexception", to: "workererror" },
    { from: "workererror", to: "resultq", label: "error payload" },
    { from: "getdata", to: "reraise", label: "unwrap and raise in main process" },
    { from: "getdata", to: "timeout", label: "no result before timeout" },
    { from: "iterator", to: "shutdown", label: "iterator exhausted or closed" },
    { from: "shutdown", to: "workerloop", label: "stop workers" },
    { from: "loader", to: "single", label: "num_workers = 0" },
    { from: "single", to: "output", label: "fetch then return directly" },
  ],
  style: { module, queue, boundary },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<N>;

export default sketch;
