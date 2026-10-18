// PostgreSQL — https://www.postgresql.org/docs/current/wal-internals.html
// https://www.postgresql.org/docs/current/storage.html
// https://www.postgresql.org/docs/current/warm-standby.html
// https://github.com/postgres/postgres/blob/master/src/backend/access/heap/heapam.c
// https://github.com/postgres/postgres/blob/master/src/backend/access/transam/xloginsert.c
// https://github.com/postgres/postgres/blob/master/src/backend/storage/buffer/bufmgr.c
// This sketch follows an INSERT from the frontend/backend protocol through heap page mutation
// and WAL insertion to commit durability, then shows page writeback and optional WAL streaming.
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
    { id: "client", class: "box", x: 20, y: 130, label: "libpq client\nQuery message: INSERT", fill: "#eff6ff" },
    { id: "server", class: "box", x: 210, y: 20, width: 720, label: "PostgreSQL instance (postmaster + server processes)", fill: "#f1f5f9" },
    { id: "postmaster", class: "box", x: 235, y: 80, parent: "server", label: "postmaster\naccepts connection; starts backend" },
    { id: "backend", class: "box", x: 235, y: 160, parent: "server", label: "client backend\nper-session process" },
    { id: "protocol", class: "box", x: 430, y: 160, parent: "server", label: "exec_simple_query\nParse / Bind / Execute path" },
    { id: "parser", class: "box", x: 430, y: 245, parent: "server", label: "raw parser + analyzer\nRawStmt → Query" },
    { id: "planner", class: "box", x: 630, y: 245, parent: "server", label: "planner / optimizer\nQuery → PlannedStmt" },
    { id: "portal", class: "box", x: 630, y: 160, parent: "server", label: "Portal / PortalRun\nexecute planned statement" },
    { id: "executor", class: "box", x: 815, y: 160, parent: "server", label: "ExecutorRun\nModifyTable → ExecInsert" },
    { id: "heap", class: "box", x: 1030, y: 160, label: "heap_insert()\nchoose target block + tuple offset", fill: "#dcfce7" },
    { id: "relcache", class: "box", x: 1030, y: 275, label: "Relation / SMgrRelation\nrelfilenode + fork metadata" },
    { id: "bufmgr", class: "box", x: 1260, y: 160, label: "ReadBuffer / LockBuffer\nshared buffer manager", fill: "#dcfce7" },
    { id: "shared", class: "box", x: 1260, y: 275, label: "shared_buffers\nbuffer table + descriptors" },
    { id: "page", class: "box", x: 1480, y: 160, label: "heap page (BLCKSZ)\nPageAddItem; mark dirty" },
    { id: "xloginsert", class: "box", x: 1480, y: 275, label: "XLogBeginInsert / XLogRegisterBuffer\nXLogInsert constructs heap-insert record", fill: "#fef3c7" },
    { id: "walshared", class: "box", x: 1720, y: 275, label: "WAL shared memory\nWAL buffers + insertion locks" },
    { id: "walwriter", class: "box", x: 1720, y: 390, label: "WAL writer\nperiodic WAL buffer write" },
    { id: "walfile", class: "box", x: 1940, y: 275, label: "pg_wal segment\nrecord at assigned LSN" },
    { id: "commit", class: "box", x: 815, y: 390, label: "CommitTransaction\nrecord transaction commit" },
    { id: "xactwal", class: "box", x: 1030, y: 390, label: "XLogInsertRecord\nXLOG_XACT_COMMIT record" },
    { id: "flush", class: "box", x: 1260, y: 390, label: "XLogFlush(commit LSN)\nwait for local WAL durability" },
    { id: "reply", class: "box", x: 430, y: 390, label: "CommandComplete + ReadyForQuery\nCommandComplete(INSERT n)" },
    { id: "datafile", class: "box", x: 1940, y: 160, label: "relation main fork\nbase/<dboid>/<relfilenode>" },
    { id: "smgr", class: "box", x: 1720, y: 160, label: "smgrwrite / smgrwritev\nphysical relation I/O" },
    { id: "bgwriter", class: "box", x: 1480, y: 500, label: "Background writer\nwrite selected dirty buffers" },
    { id: "checkpointer", class: "box", x: 1720, y: 500, label: "Checkpointer\ncheckpoint requests + fsync" },
    { id: "txstatus", class: "box", x: 1030, y: 500, label: "pg_xact (CLOG)\ntransaction commit status" },
    { id: "walsender", class: "box", x: 1940, y: 390, label: "walsender\nWAL sender process" },
    { id: "standby", class: "box", x: 1940, y: 500, label: "standby WAL receiver\nCopyData WAL stream" },
    { id: "replay", class: "box", x: 2170, y: 500, label: "startup process\nredo WAL records on standby" },
  ] satisfies Node[],
  edges: [
    { from: "client", to: "postmaster", label: "startup packet" },
    { from: "postmaster", to: "backend", label: "fork backend; socket pair" },
    { from: "client", to: "protocol", label: "Query: INSERT" },
    { from: "backend", to: "protocol", label: "frontend/backend messages" },
    { from: "protocol", to: "parser", label: "SQL text" },
    { from: "parser", to: "planner", label: "analyzed Query" },
    { from: "planner", to: "portal", label: "PlannedStmt" },
    { from: "portal", to: "executor", label: "ExecutorRun" },
    { from: "executor", to: "heap", label: "tuple + resultRelInfo" },
    { from: "heap", to: "relcache", label: "Relation metadata" },
    { from: "heap", to: "bufmgr", label: "ReadBufferExtended(relation, block)" },
    { from: "relcache", to: "bufmgr", label: "SMgrRelation / relfilenode" },
    { from: "bufmgr", to: "shared", label: "lookup or allocate buffer slot" },
    { from: "bufmgr", to: "page", label: "pinned, exclusive-locked page" },
    { from: "page", to: "heap", label: "new TID (block, offset)" },
    { from: "heap", to: "xloginsert", label: "heap insert + registered buffer" },
    { from: "xloginsert", to: "walshared", label: "assembled WAL record; LSN" },
    { from: "walshared", to: "walfile", label: "write WAL pages" },
    { from: "walwriter", to: "walshared", label: "drain WAL buffers" },
    { from: "heap", to: "commit", label: "statement complete" },
    { from: "commit", to: "xactwal", label: "commit record fields: xid, timestamp?" },
    { from: "xactwal", to: "walshared", label: "insert commit record; commit LSN" },
    { from: "commit", to: "flush", label: "synchronous_commit=on" },
    { from: "flush", to: "walfile", label: "write + fsync through commit LSN" },
    { from: "flush", to: "reply", label: "flush complete; report success" },
    { from: "reply", to: "client", label: "CommandComplete / ReadyForQuery" },
    { from: "page", to: "shared", label: "dirty buffer; page LSN" },
    { from: "shared", to: "smgr", label: "dirty page chosen for writeback" },
    { from: "smgr", to: "walfile", label: "Flush WAL through page LSN first" },
    { from: "smgr", to: "datafile", label: "write block at relation offset" },
    { from: "bgwriter", to: "shared", label: "scan and flush buffers" },
    { from: "checkpointer", to: "shared", label: "checkpoint buffer writes" },
    { from: "checkpointer", to: "walfile", label: "checkpoint record / flush ordering" },
    { from: "commit", to: "txstatus", label: "CLOG marks xid committed" },
    { from: "walfile", to: "walsender", label: "read WAL from requested LSN" },
    { from: "walsender", to: "standby", label: "WAL data messages" },
    { from: "standby", to: "replay", label: "received WAL records" },
    { from: "replay", to: "datafile", label: "redo heap changes on standby" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
