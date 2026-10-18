// PeerTube production Docker Compose deployment, based on
// https://github.com/Chocobozzz/PeerTube/blob/develop/support/docker/production/docker-compose.yml
// and https://docs.joinpeertube.org/contribute/architecture. It shows the Compose host, its
// services and mounts, plus how browser, federation, live ingest, mail and media traffic connect.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "host", class: "box", x: 300, y: 20, label: "PeerTube production Compose host\nsupport/docker/production/docker-compose.yml", width: 1100, height: 960, fill: "#f1f5f9" },
    { id: "network", class: "box", x: 350, y: 100, parent: "host", label: "Compose default network\n172.18.0.0/16 + IPv6", width: 820, height: 580, fill: "#e0f2fe" },

    { id: "browser", class: "box", x: 20, y: 180, label: "Browser / PeerTube SPA", width: 190, height: 58, fill: "#fff7ed" },
    { id: "live-encoder", class: "box", x: 20, y: 680, label: "OBS / RTMP encoder", width: 180, height: 58, fill: "#fff7ed" },
    { id: "remote-instance", class: "box", x: 1420, y: 460, label: "Remote ActivityPub\nPeerTube instances", width: 200, height: 66, fill: "#fff7ed" },
    { id: "p2p-peer", class: "box", x: 1420, y: 570, label: "Other WebTorrent\nviewers / peers", width: 200, height: 66, fill: "#fff7ed" },
    { id: "smtp-destination", class: "box", x: 1420, y: 680, label: "Recipient mail server", width: 190, height: 58, fill: "#fff7ed" },

    { id: "webserver", class: "box", x: 390, y: 140, parent: "network", label: "webserver\nNGINX :80 / :443", width: 220, height: 68, fill: "#dbeafe" },
    { id: "certbot", class: "box", x: 680, y: 140, parent: "network", label: "certbot\nrenew every 12h", width: 190, height: 68, fill: "#ede9fe" },
    { id: "reloader", class: "box", x: 930, y: 140, parent: "network", label: "webserver-reloader\nAlpine sidecar, HUP / 6h", width: 210, height: 68, fill: "#ede9fe" },
    { id: "peertube", class: "box", x: 390, y: 300, parent: "network", label: "peertube\nchocobozzz/peertube:production", width: 240, height: 72, fill: "#dcfce7" },
    { id: "postgres", class: "box", x: 700, y: 300, parent: "network", label: "postgres\npostgres:17-alpine", width: 200, height: 66, fill: "#dcfce7" },
    { id: "redis", class: "box", x: 950, y: 300, parent: "network", label: "redis\nredis:8-alpine", width: 180, height: 66, fill: "#fce7f3" },
    { id: "postfix", class: "box", x: 700, y: 500, parent: "network", label: "postfix\nmwader/postfix-relay", width: 210, height: 66, fill: "#fce7f3" },

    { id: "api", class: "box", x: 410, y: 420, parent: "network", label: "PeerTube process: Express REST API\ncontrollers + database", width: 220, height: 58, fill: "#eff6ff" },
    { id: "activitypub", class: "box", x: 410, y: 500, parent: "network", label: "PeerTube process: ActivityPub API\ninbox / outbox", width: 220, height: 58, fill: "#eff6ff" },
    { id: "tracker", class: "box", x: 410, y: 580, parent: "network", label: "PeerTube process: WebTorrent tracker\nWebSocket signaling", width: 220, height: 58, fill: "#eff6ff" },
    { id: "scheduler", class: "box", x: 900, y: 420, parent: "network", label: "PeerTube process: task scheduler + worker\nBullMQ jobs; ffmpeg", width: 230, height: 64, fill: "#eff6ff" },
    { id: "rtmp", class: "box", x: 900, y: 520, parent: "network", label: "PeerTube process: RTMP listener :1935\noptional live input", width: 230, height: 60, fill: "#eff6ff" },

    { id: "assets", class: "box", x: 390, y: 710, parent: "host", label: "assets (named volume)\n/app/client/dist ↔ web root (read-only)", width: 250, height: 62, fill: "#fef3c7" },
    { id: "data", class: "box", x: 680, y: 710, parent: "host", label: "./docker-volume/data\npeertube /data; NGINX storage", width: 230, height: 62, fill: "#fef3c7" },
    { id: "config", class: "box", x: 950, y: 710, parent: "host", label: "./docker-volume/config\npeertube /config", width: 210, height: 62, fill: "#fef3c7" },
    { id: "dbdata", class: "box", x: 390, y: 800, parent: "host", label: "./docker-volume/db\nPostgreSQL data directory", width: 220, height: 62, fill: "#fef3c7" },
    { id: "redisdata", class: "box", x: 640, y: 800, parent: "host", label: "./docker-volume/redis\nRedis /data", width: 210, height: 62, fill: "#fef3c7" },
    { id: "certconf", class: "box", x: 880, y: 800, parent: "host", label: "./docker-volume/certbot/conf\nTLS certificates", width: 230, height: 62, fill: "#fef3c7" },
    { id: "certweb", class: "box", x: 1130, y: 710, parent: "host", label: "certbot-www\nshared ACME webroot", width: 210, height: 62, fill: "#fef3c7" },
    { id: "nginxlogs", class: "box", x: 1130, y: 800, parent: "host", label: "./docker-volume/nginx-logs\nNGINX logs", width: 220, height: 62, fill: "#fef3c7" },
    { id: "dkim", class: "box", x: 880, y: 890, parent: "host", label: "./docker-volume/opendkim/keys\nPostfix DKIM keys", width: 230, height: 62, fill: "#fef3c7" },
  ] satisfies Node[],
  edges: [
    { from: "browser", to: "webserver", label: "HTTPS :443 / HTTP :80" },
    { from: "webserver", to: "api", label: "proxy REST / Socket.IO to peertube:9000" },
    { from: "webserver", to: "browser", label: "SPA assets + video HTTP segments" },
    { from: "browser", to: "tracker", label: "WebTorrent WebSocket; announce / peer info" },
    { from: "browser", to: "p2p-peer", label: "WebRTC peer video segments" },
    { from: "p2p-peer", to: "browser", label: "shared video segments" },
    { from: "api", to: "postgres", label: "Sequelize metadata and accounts" },
    { from: "api", to: "redis", label: "route cache + BullMQ enqueue" },
    { from: "scheduler", to: "redis", label: "consume BullMQ jobs" },
    { from: "scheduler", to: "data", label: "read uploads; write transcoded WebVideo / HLS" },
    { from: "api", to: "data", label: "video files and thumbnails" },
    { from: "webserver", to: "data", label: "serve mounted storage files" },
    { from: "api", to: "postfix", label: "send instance mail via relay" },
    { from: "postfix", to: "smtp-destination", label: "SMTP relay" },
    { from: "postfix", to: "dkim", label: "read signing keys" },
    { from: "activitypub", to: "remote-instance", label: "federated HTTP activities" },
    { from: "remote-instance", to: "activitypub", label: "inbox activities" },
    { from: "live-encoder", to: "rtmp", label: "RTMP :1935 published by host" },
    { from: "rtmp", to: "scheduler", label: "stream input for ffmpeg → HLS" },
    { from: "api", to: "assets", label: "populate client distribution" },
    { from: "webserver", to: "assets", label: "read-only SPA mount" },
    { from: "peertube", to: "config", label: "/config production settings" },
    { from: "postgres", to: "dbdata", label: "mount /var/lib/postgresql/data" },
    { from: "redis", to: "redisdata", label: "mount /data" },
    { from: "webserver", to: "certconf", label: "read /etc/letsencrypt" },
    { from: "certbot", to: "certconf", label: "renew and persist certificates" },
    { from: "certbot", to: "certweb", label: "write ACME challenge files" },
    { from: "webserver", to: "certweb", label: "serve /.well-known/acme-challenge" },
    { from: "webserver", to: "nginxlogs", label: "write access / error logs" },
    { from: "reloader", to: "webserver", label: "shared PID namespace; kill -HUP 1" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
