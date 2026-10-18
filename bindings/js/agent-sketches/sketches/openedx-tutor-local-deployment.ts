// Open edX deployed locally by Tutor. Sources: https://github.com/overhangio/tutor/blob/release/docs/local.rst and https://github.com/overhangio/tutor/blob/release/tutor/templates/local/docker-compose.yml. This sketch follows the single-server Compose deployment from browser ingress through LMS and Studio to their databases, search, media, cache and mail dependencies, including the files and host data directories mounted into the containers.
import type { Sketch } from "nicevg";
import { svg, text } from "nicevg";

type N = { id: string; class: "area" | "service" | "record" | "outside"; x: number; y: number; parent?: string; title?: string; fields?: string[]; width?: number; height?: number };
const area = ({ title, width = 520, height = 660 }: N) => svg`<rect data-bounds="true" width="${width}" height="${height}" rx="12" fill="#f1f5f9" stroke="#64748b"/>${text(title ?? "", 15).at({ left: 14, top: 8 })}`;
const service = ({ title }: N) => { const t = text(title ?? "", 13); const w = Math.max(150, t.width + 26); return svg`<rect data-bounds="true" width="${w}" height="52" rx="12" fill="#dbeafe" stroke="#1d4ed8"/>${t.at({ cx: w / 2, cy: 26 })}`; };
const record = ({ title, fields = [] }: N) => { const lines = [text(title ?? "", 13), ...fields.map((f) => text(f, 11))]; const w = Math.max(185, ...lines.map((l) => l.width + 20)); const h = 28 + fields.length * 17; return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="2" fill="#fff7ed" stroke="#c2410c"/><rect width="${w}" height="28" fill="#fed7aa" stroke="#c2410c"/>${lines[0]!.at({ left: 10, cy: 14 })}${lines.slice(1).map((l, i) => l.at({ left: 10, top: 31 + i * 17 }))}`; };
const outside = ({ title }: N) => { const t = text(title ?? "", 13); const w = Math.max(150, t.width + 28); return svg`<path data-bounds="true" d="M 16 0 H ${w - 16} L ${w} 26 L ${w - 16} 52 H 16 L 0 26 Z" fill="#fef9c3" stroke="#a16207"/>${t.at({ cx: w / 2, cy: 26 })}`; };

const nodes: N[] = [
  { id: "host", class: "area", x: 30, y: 30, title: "One Linux host · Docker Compose local mode", width: 1280, height: 860 },
  { id: "browser", class: "outside", x: 30, y: 120, title: "Learner / author browser" },
  { id: "caddy", class: "service", parent: "host", x: 260, y: 115, title: "Caddy · web proxy" },
  { id: "lms", class: "service", parent: "host", x: 500, y: 100, title: "lms · SERVICE_VARIANT=lms" },
  { id: "cms", class: "service", parent: "host", x: 500, y: 205, title: "cms · SERVICE_VARIANT=cms" },
  { id: "lmscfg", class: "record", parent: "host", x: 480, y: 320, title: "settings/lms", fields: ["DJANGO_SETTINGS_MODULE", "lms.envs.tutor.production", "UWSGI_WORKERS"] },
  { id: "cmscfg", class: "record", parent: "host", x: 720, y: 320, title: "settings/cms", fields: ["cms.envs.tutor.production", "UWSGI_WORKERS"] },
  { id: "openedxcfg", class: "record", parent: "host", x: 960, y: 320, title: "apps/openedx/config", fields: ["generated Tutor settings", "shared by lms and cms"] },
  { id: "mysql", class: "service", parent: "host", x: 250, y: 445, title: "mysql · MySQL 8.4" },
  { id: "mysqlvol", class: "record", parent: "host", x: 250, y: 530, title: "data/mysql", fields: ["host bind mount → /var/lib/mysql", "openedx database · utf8mb4"] },
  { id: "mongo", class: "service", parent: "host", x: 500, y: 445, title: "mongodb · WiredTiger" },
  { id: "mongovol", class: "record", parent: "host", x: 500, y: 530, title: "data/mongodb", fields: ["host bind mount → /data/db", "openedx database"] },
  { id: "redis", class: "service", parent: "host", x: 750, y: 445, title: "redis · cache and Celery broker" },
  { id: "redisvol", class: "record", parent: "host", x: 750, y: 530, title: "data/redis", fields: ["host bind mount → /openedx/redis/data", "cache DB 1 · celery DB 0"] },
  { id: "redisconf", class: "record", parent: "host", x: 750, y: 630, title: "apps/redis/redis.conf", fields: ["read-only config bind mount"] },
  { id: "meili", class: "service", parent: "host", x: 1010, y: 445, title: "meilisearch · :7700" },
  { id: "meilivol", class: "record", parent: "host", x: 1010, y: 530, title: "data/meilisearch", fields: ["host bind mount → /meili_data", "course search index"] },
  { id: "smtp", class: "service", parent: "host", x: 260, y: 680, title: "smtp · Exim relay" },
  { id: "media", class: "record", parent: "host", x: 500, y: 680, title: "data/openedx-media", fields: ["shared /openedx/media", "course and user uploads"] },
  { id: "private", class: "record", parent: "host", x: 750, y: 680, title: "data/openedx-media-private", fields: ["shared /openedx/media-private", "private uploads"] },
  { id: "lmsdata", class: "record", parent: "host", x: 1010, y: 680, title: "data/lms", fields: ["host bind mount → /openedx/data", "LMS runtime data"] },
  { id: "worker", class: "service", parent: "host", x: 30, y: 795, title: "lms-worker · Celery" },
  { id: "cmsworker", class: "service", parent: "host", x: 275, y: 795, title: "cms-worker · Celery" },
  { id: "jobs", class: "record", parent: "host", x: 520, y: 795, title: "docker-compose.jobs.yml", fields: ["lms-job / cms-job one-shot tasks", "migrations and initialization"] },
  { id: "permissions", class: "service", parent: "host", x: 770, y: 795, title: "permissions · setowners.sh" },
  { id: "mfe", class: "outside", x: 1010, y: 120, title: "Open edX MFE assets / clients" },
  { id: "email", class: "outside", x: 1010, y: 220, title: "Configured SMTP destination" },
  { id: "catalog", class: "outside", x: 30, y: 300, title: "Optional external course catalog" },
  { id: "mysqlcfg", class: "record", parent: "host", x: 30, y: 445, title: "Tutor config.yml · DB fields", fields: ["MYSQL_HOST=mysql", "OPENEDX_MYSQL_DATABASE", "OPENEDX_MYSQL_USERNAME"] },
  { id: "mongocfg", class: "record", parent: "host", x: 30, y: 555, title: "Tutor config.yml · MongoDB", fields: ["MONGODB_HOST=mongodb", "MONGODB_DATABASE=openedx", "MONGODB_PORT=27017"] },
  { id: "searchcfg", class: "record", parent: "host", x: 30, y: 665, title: "Tutor config.yml · Meilisearch", fields: ["MEILISEARCH_URL", "MEILISEARCH_INDEX_PREFIX"] },
  { id: "public", class: "outside", x: 30, y: 205, title: "DNS names: LMS_HOST / CMS_HOST" },
  { id: "static", class: "record", parent: "host", x: 960, y: 795, title: "apps/openedx/uwsgi.ini", fields: ["read-only bind mount", "shared by lms and cms"] },
];

const edges = [
  { from: "browser", to: "caddy", label: "HTTPS / LMS_HOST or CMS_HOST" },
  { from: "public", to: "caddy", label: "DNS routes both hostnames" },
  { from: "caddy", to: "lms", label: "LMS requests" }, { from: "caddy", to: "cms", label: "Studio requests" },
  { from: "lms", to: "lmscfg", label: "mount settings" }, { from: "cms", to: "cmscfg", label: "mount settings" },
  { from: "lms", to: "openedxcfg", label: "shared config mount" }, { from: "cms", to: "openedxcfg", label: "shared config mount" },
  { from: "lms", to: "mysql", label: "Django relational records · mysql:3306" }, { from: "cms", to: "mysql", label: "Studio records · mysql:3306" },
  { from: "lms", to: "mongo", label: "content/course documents · mongodb:27017" }, { from: "cms", to: "mongo", label: "course authoring data" },
  { from: "lms", to: "redis", label: "cache DB 1 · Celery DB 0" }, { from: "cms", to: "redis", label: "cache and task queue" },
  { from: "lms", to: "meili", label: "course search API · :7700" }, { from: "cms", to: "meili", label: "index course metadata" },
  { from: "lms", to: "smtp", label: "outbound email · smtp:8025" }, { from: "cms", to: "smtp", label: "authoring notifications" },
  { from: "smtp", to: "email", label: "SMTP relay" },
  { from: "mysql", to: "mysqlvol", label: "/var/lib/mysql" }, { from: "mongo", to: "mongovol", label: "/data/db" },
  { from: "redis", to: "redisvol", label: "RDB/AOF data" }, { from: "redis", to: "redisconf", label: "redis-server config" },
  { from: "meili", to: "meilivol", label: "/meili_data" },
  { from: "lms", to: "media", label: "/openedx/media" }, { from: "cms", to: "media", label: "shared course uploads" },
  { from: "lms", to: "private", label: "/openedx/media-private" }, { from: "cms", to: "private", label: "private uploads" },
  { from: "lms", to: "lmsdata", label: "/openedx/data" },
  { from: "redis", to: "worker", label: "queued LMS tasks" }, { from: "worker", to: "mysql", label: "task results / records" },
  { from: "redis", to: "cmsworker", label: "queued CMS tasks" }, { from: "cmsworker", to: "mysql", label: "task results / records" },
  { from: "jobs", to: "mysql", label: "lms-job / cms-job migrations" }, { from: "permissions", to: "mysqlvol", label: "set host bind-mount ownership" },
  { from: "permissions", to: "mongovol", label: "set ownership" }, { from: "permissions", to: "media", label: "set ownership" },
  { from: "mfe", to: "caddy", label: "served frontend assets / API calls" }, { from: "lms", to: "catalog", label: "catalog integration API (optional)" },
  { from: "lms", to: "static", label: "uWSGI configuration mount" }, { from: "cms", to: "static", label: "uWSGI configuration mount" },
  { from: "lms", to: "searchcfg", label: "MEILISEARCH_URL / prefix" }, { from: "lms", to: "mysqlcfg", label: "database connection settings" },
  { from: "lms", to: "mongocfg", label: "document store settings" },
];

export default { nodes, edges, style: { area, service, record, outside }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<N>;
