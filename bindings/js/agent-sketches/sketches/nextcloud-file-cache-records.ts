// Nextcloud file-cache records, based on:
// https://docs.nextcloud.com/server/stable/admin_manual/occ_files.html
// https://docs.nextcloud.com/server/stable/admin_manual/gdpr/data_retention.html
// https://github.com/nextcloud/server/blob/master/lib/private/Files/Cache/Cache.php
// https://github.com/nextcloud/server/blob/master/lib/private/Files/Utils/Scanner.php
// https://github.com/nextcloud/server/blob/master/apps/files_trashbin/lib/Trashbin.php
// https://github.com/nextcloud/server/blob/master/lib/private/Share20/DefaultShareProvider.php
// The sketch follows a user's file records through storage, folder, share, version,
// and trash relationships, then shows a stale-cache recovery after an out-of-band move.
import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = { id: string; class: "box"; x: number; y: number; parent?: string; label: string; fill?: string };
const nodes: Node[] = [
  { id: "user", class: "box", x: 30, y: 40, label: "User\noc_users: uid (PK)" },
  { id: "group", class: "box", x: 30, y: 190, label: "Group\noc_groups: gid (PK)" },
  { id: "mount", class: "box", x: 300, y: 40, label: "Mount\noc_mounts: mount_id, user_id,\nstorage_id, root_id, mount_point" },
  { id: "storage", class: "box", x: 590, y: 40, label: "Storage\noc_storages: numeric id, id string" },
  { id: "root-folder", class: "box", x: 850, y: 40, label: "Root folder CacheEntry\nfileid (PK), storage, parent, path, name" },
  { id: "filecache", class: "box", x: 1160, y: 40, label: "File Cache · oc_filecache\nfileid (PK), storage, path, path_hash,\nparent, name, mimetype, size, mtime,\netag, permissions, checksum", fill: "#eff6ff" },
  { id: "child", class: "box", x: 1160, y: 220, label: "Child CacheEntry\nfileid (PK), parent → folder.fileid,\npath, name, size, mtime, etag" },
  { id: "metadata", class: "box", x: 1480, y: 40, label: "File metadata\noc_files_metadata: file_id (FK),\nmetadata json, sync_token" },
  { id: "share", class: "box", x: 1480, y: 220, label: "Share\noc_share: id, share_type,\nuid_owner, file_source, permissions" },
  { id: "recipient", class: "box", x: 1760, y: 220, label: "Recipient\nuser or group receiving share" },
  { id: "version", class: "box", x: 850, y: 410, label: "File version\nfiles_versions storage path +\nversion timestamp/name" },
  { id: "trash", class: "box", x: 1160, y: 410, label: "Deleted file\nfiles_trashbin/files path +\noriginal name and deletion time" },
  { id: "external-change", class: "box", x: 30, y: 410, label: "Admin / filesystem tool\nMoves a file outside Nextcloud", fill: "#fff7ed" },
  { id: "stale", class: "box", x: 300, y: 410, label: "Failure: stale file cache\nold path remains; actual path differs", fill: "#fee2e2" },
  { id: "scan", class: "box", x: 30, y: 650, label: "Admin runs\nocc files:scan --path=\"/uid/files\"", fill: "#f0fdf4" },
  { id: "scanner", class: "box", x: 350, y: 650, label: "Scanner\nwalks storage; updates cache\nfor new or changed files" },
  { id: "actual", class: "box", x: 700, y: 650, label: "Actual storage\nfilesystem / mounted backend\nfiles and folders" },
  { id: "repaired", class: "box", x: 1160, y: 650, label: "Reconciled file cache\nnew path/name/parent metadata;\nfile ID reused when same file is found", fill: "#dcfce7" },
  { id: "cleanup", class: "box", x: 1480, y: 650, label: "occ files:cleanup\nremoves cache rows with no matching\nstorage entries (separate stale-row case)" },
];

const edges = [
  { from: "mount", to: "user", label: "user_id" },
  { from: "mount", to: "storage", label: "storage_id" },
  { from: "mount", to: "root-folder", label: "root_id = fileid" },
  { from: "root-folder", to: "filecache", label: "root is a cache entry" },
  { from: "filecache", to: "storage", label: "storage ID scopes fileid" },
  { from: "child", to: "root-folder", label: "parent = parent folder fileid" },
  { from: "child", to: "filecache", label: "each file/folder is a row" },
  { from: "metadata", to: "child", label: "file_id" },
  { from: "share", to: "child", label: "file_source = fileid" },
  { from: "share", to: "recipient", label: "share_with / share_type" },
  { from: "group", to: "recipient", label: "group recipient by gid" },
  { from: "version", to: "child", label: "prior content copies for file" },
  { from: "trash", to: "child", label: "deleted file moved to trashbin" },
  { from: "external-change", to: "actual", label: "move outside Nextcloud" },
  { from: "external-change", to: "stale", label: "cache not updated in-app" },
  { from: "stale", to: "filecache", label: "stored path no longer matches" },
  { from: "scan", to: "scanner", label: "manual scan request" },
  { from: "scanner", to: "actual", label: "enumerate files and metadata" },
  { from: "scanner", to: "repaired", label: "recreate/update cache entries" },
  { from: "repaired", to: "filecache", label: "reconciled rows" },
  { from: "cleanup", to: "filecache", label: "remove orphan cache rows" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
