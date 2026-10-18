// Jellyfin — https://github.com/jellyfin/jellyfin/blob/master/Jellyfin.Api/Controllers/DynamicHlsController.cs
// https://github.com/jellyfin/jellyfin/blob/master/Jellyfin.Api/Helpers/StreamingHelpers.cs
// https://jellyfin.org/docs/general/post-install/transcoding/
// A client's video HLS playlist request enters DynamicHlsController, which resolves the source and starts FFmpeg when needed. Jellyfin waits for initial segments and returns playlist text; the media file and generated HLS files cross the server's storage boundary.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  width?: number;
  height?: number;
  label: string;
  fill?: string;
};

const nodes: Node[] = [
  { id: "client", class: "box", x: 20, y: 210, width: 160, height: 68, label: "Jellyfin client\n(player)" },
  { id: "server", class: "box", x: 230, y: 60, width: 1180, height: 600, label: "Jellyfin server process", fill: "#f8fafc" },
  { id: "endpoint", parent: "server", class: "box", x: 270, y: 160, width: 175, height: 76, label: "ASP.NET Core\nrouting + authorization" },
  { id: "controller", parent: "server", class: "box", x: 500, y: 130, width: 205, height: 82, label: "DynamicHlsController\nvideo playlist action" },
  { id: "dto", parent: "server", class: "box", x: 500, y: 270, width: 205, height: 76, label: "StreamingRequestDto\nroute + query values" },
  { id: "helpers", parent: "server", class: "box", x: 760, y: 250, width: 205, height: 90, label: "StreamingHelpers\n.GetStreamingState" },
  { id: "library", parent: "server", class: "box", x: 1010, y: 140, width: 170, height: 66, label: "ILibraryManager\nGetItemById" },
  { id: "sources", parent: "server", class: "box", x: 1010, y: 250, width: 180, height: 76, label: "IMediaSourceManager\nselect media source" },
  { id: "encoder", parent: "server", class: "box", x: 1010, y: 390, width: 180, height: 76, label: "IMediaEncoder\ncodec capability checks" },
  { id: "state", parent: "server", class: "box", x: 760, y: 410, width: 205, height: 72, label: "StreamState\nsource + output paths" },
  { id: "manager", parent: "server", class: "box", x: 500, y: 430, width: 205, height: 76, label: "ITranscodeManager\nlock + StartFfMpeg" },
  { id: "ffmpeg", class: "box", x: 500, y: 570, width: 205, height: 72, label: "Jellyfin FFmpeg\nHLS mux / transcode" },
  { id: "storage", class: "box", x: 750, y: 710, width: 560, height: 250, label: "Media and transcode storage", fill: "#fff7ed" },
  { id: "media", parent: "storage", class: "box", x: 790, y: 800, width: 205, height: 70, label: "Library source\nvideo file" },
  { id: "output", parent: "storage", class: "box", x: 1050, y: 800, width: 210, height: 84, label: "Transcode directory\n.m3u8 + HLS segments" },
  { id: "wait", parent: "server", class: "box", x: 270, y: 330, width: 190, height: 76, label: "HlsHelpers\nWaitForMinimumSegmentCount" },
  { id: "playlist", parent: "server", class: "box", x: 270, y: 490, width: 190, height: 76, label: "HlsHelpers\nGetLivePlaylistText" },
  { id: "response", parent: "server", class: "box", x: 270, y: 520, width: 190, height: 76, label: "HTTP Content response\nplaylist.m3u8" },
];

const sketch = {
  nodes,
  edges: [
    { from: "client", to: "endpoint", label: "GET video HLS playlist\nitemId + playback options" },
    { from: "endpoint", to: "controller", label: "authorized route" },
    { from: "controller", to: "dto", label: "bind path + query" },
    { from: "dto", to: "helpers", label: "streaming request" },
    { from: "helpers", to: "library", label: "item ID → BaseItem" },
    { from: "helpers", to: "sources", label: "resolve MediaSourceInfo" },
    { from: "helpers", to: "encoder", label: "validate requested codecs" },
    { from: "helpers", to: "state", label: "initialize request, user, source" },
    { from: "controller", to: "state", label: "GetStreamingState result" },
    { from: "state", to: "manager", label: "output path + FFmpeg arguments" },
    { from: "manager", to: "ffmpeg", label: "start process if playlist absent" },
    { from: "ffmpeg", to: "media", label: "read source" },
    { from: "ffmpeg", to: "output", label: "write playlist + segments" },
    { from: "state", to: "output", label: "OutputFilePath → .m3u8" },
    { from: "output", to: "wait", label: "playlist path; wait for minSegments" },
    { from: "wait", to: "playlist", label: "segments ready" },
    { from: "playlist", to: "response", label: "playlist text" },
    { from: "response", to: "client", label: "200 playlist.m3u8" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
