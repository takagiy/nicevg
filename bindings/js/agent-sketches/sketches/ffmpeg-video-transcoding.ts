// FFmpeg: https://www.ffmpeg.org/ffmpeg.html
// A single video stream's packets travel from an input container through decoding,
// filtering and encoding, then the muxer writes the output container. The bypass shows streamcopy.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "input", class: "box", x: 30, y: 90, label: "入力ファイル\nコンテナ", fill: "#fef3c7" },
    { id: "demuxer", class: "box", x: 230, y: 65, label: "demuxer\n(libavformat)" },
    { id: "decoder", class: "box", x: 435, y: 115, label: "decoder\n(libavcodec)" },
    { id: "filter", class: "box", x: 630, y: 70, label: "video filtergraph\n(libavfilter)" },
    { id: "encoder", class: "box", x: 850, y: 130, label: "encoder\n(libavcodec)" },
    { id: "muxer", class: "box", x: 1040, y: 70, label: "muxer\n(libavformat)" },
    { id: "output", class: "box", x: 1240, y: 115, label: "出力ファイル\nコンテナ", fill: "#dcfce7" },
  ] satisfies Node[],
  edges: [
    { from: "input", to: "demuxer", label: "コンテナバイト列" },
    { from: "demuxer", to: "decoder", label: "圧縮 video packets" },
    { from: "decoder", to: "filter", label: "raw video frames" },
    { from: "filter", to: "encoder", label: "加工済み frames" },
    { from: "encoder", to: "muxer", label: "圧縮 packets" },
    { from: "muxer", to: "output", label: "多重化したバイト列" },
    { from: "demuxer", to: "muxer", label: "-c copy: packet をそのまま渡す" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
