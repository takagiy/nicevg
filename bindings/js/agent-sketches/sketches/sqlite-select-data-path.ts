// SQLite's SELECT data path, based on the official architecture and bytecode documentation.
// Follows SQL from preparation through VDBE execution and page reads to rows returned by sqlite3_step/sqlite3_column.
// Sources: https://www.sqlite.org/arch.html
//          https://www.sqlite.org/opcode.html
//          https://www.sqlite.org/c3ref/prepare.html
//          https://www.sqlite.org/c3ref/step.html
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const sketch = {
  nodes: [
    { id: "caller", class: "box", x: 40, y: 190, label: "Application\nSQL text + bind value\nSELECT name FROM users WHERE id = ?1", width: 270, height: 84 },
    { id: "prepare", class: "box", x: 370, y: 170, label: "sqlite3_prepare_v2()\ncompile SQL to sqlite3_stmt", width: 230, height: 70 },
    { id: "tokenizer", class: "box", x: 670, y: 40, label: "tokenize.c\nSQL characters → tokens", width: 210, height: 64 },
    { id: "parser", class: "box", x: 950, y: 40, label: "parse.y / Lemon parser\ntokens → parse tree", width: 230, height: 68 },
    { id: "codegen", class: "box", x: 1240, y: 40, label: "Code generator + planner\nselect.c · where.c · expr.c\nparse tree → VDBE bytecode", width: 280, height: 84 },
    { id: "stmt", class: "box", x: 1590, y: 165, label: "sqlite3_stmt\nprepared VDBE program", width: 220, height: 66 },
    { id: "bind", class: "box", x: 370, y: 340, label: "sqlite3_bind_int(stmt, 1, id)\nparameter value stored for run", width: 250, height: 68 },
    { id: "step", class: "box", x: 700, y: 330, label: "sqlite3_step(stmt)\nstart / resume execution", width: 220, height: 68 },
    { id: "vdbe", class: "box", x: 1010, y: 310, label: "VDBE (vdbe.c)\nexecutes opcodes; Mem values\nOPENREAD · SEEKROWID · COLUMN · RESULTROW", width: 310, height: 88 },
    { id: "btree", class: "box", x: 1400, y: 310, label: "B-Tree (btree.c)\nrowid table / index cursors\nfind row and read record fields", width: 260, height: 82 },
    { id: "pager", class: "box", x: 1400, y: 480, label: "Pager + page cache (pager.c)\npage lookup; cache hit or disk read\ntransaction and journal handling", width: 290, height: 86 },
    { id: "vfs", class: "box", x: 1050, y: 520, label: "OS Interface / VFS\nfile xRead and locking methods", width: 240, height: 66 },
    { id: "db", class: "box", x: 1770, y: 490, label: "Database file\nB-tree pages", width: 180, height: 62 },
    { id: "rows", class: "box", x: 700, y: 490, label: "sqlite3_step() → SQLITE_ROW\nsqlite3_column_text(stmt, 0)\nname value → application", width: 280, height: 80 },
    { id: "done", class: "box", x: 370, y: 510, label: "sqlite3_step() → SQLITE_DONE\nno more rows (or error code)", width: 250, height: 68 },
  ],
  edges: [
    { from: "caller", to: "prepare", label: "SQL string" },
    { from: "prepare", to: "tokenizer", label: "SQL text" },
    { from: "tokenizer", to: "parser", label: "tokens" },
    { from: "parser", to: "codegen", label: "parse tree" },
    { from: "codegen", to: "stmt", label: "compiled bytecode" },
    { from: "prepare", to: "caller", label: "sqlite3_stmt*" },
    { from: "caller", to: "bind", label: "parameter ?1 value" },
    { from: "bind", to: "stmt", label: "bound value in stmt" },
    { from: "caller", to: "step", label: "step prepared statement" },
    { from: "step", to: "vdbe", label: "run / resume bytecode" },
    { from: "stmt", to: "vdbe", label: "program + bindings" },
    { from: "vdbe", to: "btree", label: "cursor seek / column read" },
    { from: "btree", to: "pager", label: "page number; page bytes" },
    { from: "pager", to: "vfs", label: "cache miss: xRead" },
    { from: "vfs", to: "db", label: "read database page" },
    { from: "db", to: "vfs", label: "page bytes" },
    { from: "vfs", to: "pager", label: "page bytes into cache" },
    { from: "pager", to: "btree", label: "page content" },
    { from: "btree", to: "vdbe", label: "record / column value" },
    { from: "vdbe", to: "step", label: "SQLITE_ROW + result registers" },
    { from: "step", to: "rows", label: "row available" },
    { from: "rows", to: "caller", label: "column value" },
    { from: "step", to: "done", label: "SQLITE_DONE / error" },
    { from: "done", to: "caller", label: "completion status" },
  ],
  style: { box },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string; width: number; height: number }>;

export default sketch;
