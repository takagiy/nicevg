// PostgreSQL executor internals, based on:
// https://github.com/postgres/postgres/blob/master/src/backend/executor/README
// https://github.com/postgres/postgres/blob/master/src/backend/executor/execProcnode.c
// https://www.postgresql.org/docs/current/planner-optimizer.html
// This sketch follows a representative SELECT plan through executor setup, pull-based tuple
// production, heap access, expression evaluation, result delivery, and resource cleanup.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "boundary";
  x: number;
  y: number;
  label: string;
  parent?: string;
  width?: number;
  height?: number;
  fill?: string;
};

const nodes: Node[] = [
  { id: "frontend", class: "box", x: 20, y: 35, label: "Frontend\n(client)" },
  { id: "backend", class: "boundary", x: 190, y: 20, width: 900, height: 1050, label: "PostgreSQL backend process" },
  { id: "querydesc", class: "box", x: 220, y: 55, label: "QueryDesc\nplannedstmt + params" },
  { id: "start", class: "box", x: 450, y: 50, label: "ExecutorStart" },
  { id: "perms", class: "box", x: 670, y: 40, label: "ExecCheckRTPerms\nrelation privileges" },
  { id: "estate", class: "box", x: 670, y: 120, label: "EState\nshared execution state" },
  { id: "queryctx", class: "box", x: 900, y: 120, label: "per-query memory context" },
  { id: "init", class: "box", x: 450, y: 175, label: "ExecInitNode\nrecursive initialization" },
  { id: "plan", class: "boundary", x: 220, y: 250, width: 850, height: 400, label: "PlanState tree (representative SELECT)" },
  { id: "limit", class: "box", x: 560, y: 290, label: "LimitState\nExecLimit" , parent: "plan" },
  { id: "sort", class: "box", x: 560, y: 375, label: "SortState\nExecSort", parent: "plan" },
  { id: "join", class: "box", x: 560, y: 465, label: "HashJoinState\nExecHashJoin", parent: "plan" },
  { id: "seqorders", class: "box", x: 290, y: 555, label: "SeqScanState\norders", parent: "plan" },
  { id: "hash", class: "box", x: 810, y: 555, label: "HashState\nExecHash", parent: "plan" },
  { id: "seqcustomers", class: "box", x: 810, y: 600, label: "SeqScanState\ncustomers", parent: "plan" },
  { id: "run", class: "box", x: 450, y: 700, label: "ExecutorRun" },
  { id: "execute", class: "box", x: 450, y: 770, label: "ExecutePlan" },
  { id: "procnode", class: "box", x: 450, y: 840, label: "ExecProcNode\nrequest next tuple" },
  { id: "slot", class: "box", x: 220, y: 920, label: "TupleTableSlot\nvirtual tuple" },
  { id: "exprctx", class: "box", x: 220, y: 1000, label: "ExprContext\nper-tuple context" },
  { id: "qual", class: "box", x: 450, y: 920, label: "ExecQual\njoin / filter clauses" },
  { id: "eval", class: "box", x: 450, y: 1000, label: "ExecEvalExpr\nprojection + expressions" },
  { id: "tableam", class: "box", x: 810, y: 710, label: "Table AM\nheap_getnextslot" },
  { id: "buffer", class: "box", x: 810, y: 795, label: "ReadBuffer /\nshared buffers" },
  { id: "heap", class: "box", x: 810, y: 880, label: "heap relation\nblocks + tuples" },
  { id: "dest", class: "box", x: 670, y: 925, label: "DestReceiver\nreceive tuples" },
  { id: "finish", class: "box", x: 670, y: 1005, label: "ExecutorFinish\ntrigger completion" },
  { id: "end", class: "box", x: 900, y: 1005, label: "ExecutorEnd" },
  { id: "endnode", class: "box", x: 900, y: 925, label: "ExecEndNode\nclose scans, release pins" },
  { id: "aftertrigger", class: "box", x: 900, y: 845, label: "AfterTriggerEndQuery" },
  { id: "complete", class: "box", x: 20, y: 165, label: "CommandComplete /\nprotocol result" },
  { id: "queryfree", class: "box", x: 670, y: 845, label: "FreeExecutorState\nfree query context" },
];

const edges = [
  { from: "frontend", to: "querydesc", label: "planned query + params" },
  { from: "querydesc", to: "start", label: "QueryDesc" },
  { from: "start", to: "perms", label: "check permissions" },
  { from: "start", to: "estate", label: "CreateExecutorState" },
  { from: "estate", to: "queryctx", label: "per-query allocations" },
  { from: "start", to: "init", label: "initialize plan" },
  { from: "init", to: "plan", label: "ExecInitNode builds PlanState tree" },
  { from: "init", to: "estate", label: "shared EState" },
  { from: "limit", to: "sort", label: "child plan" },
  { from: "sort", to: "join", label: "child plan" },
  { from: "join", to: "seqorders", label: "outerPlanState" },
  { from: "join", to: "hash", label: "innerPlanState" },
  { from: "hash", to: "seqcustomers", label: "build hash table from rows" },
  { from: "run", to: "execute", label: "run direction + count" },
  { from: "execute", to: "procnode", label: "pull root tuple repeatedly" },
  { from: "procnode", to: "limit", label: "ExecProcNode(root)" },
  { from: "limit", to: "sort", label: "request tuples" },
  { from: "sort", to: "join", label: "request / sort input tuples" },
  { from: "join", to: "seqorders", label: "outer tuples" },
  { from: "join", to: "hash", label: "probe with outer key" },
  { from: "seqorders", to: "tableam", label: "scan relation" },
  { from: "seqcustomers", to: "tableam", label: "scan relation" },
  { from: "tableam", to: "buffer", label: "block read / pin" },
  { from: "buffer", to: "heap", label: "page contents" },
  { from: "heap", to: "slot", label: "tuple values" },
  { from: "seqorders", to: "slot", label: "scan slot" },
  { from: "seqcustomers", to: "slot", label: "scan slot" },
  { from: "join", to: "qual", label: "join qualification" },
  { from: "qual", to: "exprctx", label: "current outer / inner slots" },
  { from: "exprctx", to: "eval", label: "expression environment" },
  { from: "eval", to: "join", label: "match result / projected values" },
  { from: "join", to: "slot", label: "joined row in result slot" },
  { from: "slot", to: "execute", label: "tuple to destination" },
  { from: "execute", to: "dest", label: "receiveSlot" },
  { from: "dest", to: "frontend", label: "row description + DataRow" },
  { from: "run", to: "finish", label: "run exhausted" },
  { from: "finish", to: "aftertrigger", label: "AfterTriggerEndQuery" },
  { from: "finish", to: "end", label: "ExecutorEnd" },
  { from: "end", to: "endnode", label: "walk plan state tree" },
  { from: "endnode", to: "plan", label: "ExecEndNode recursively" },
  { from: "endnode", to: "queryfree", label: "resources closed" },
  { from: "queryfree", to: "queryctx", label: "free context tree" },
  { from: "end", to: "complete", label: "protocol completion" },
];

const sketch = {
  nodes,
  edges,
  style: {
    box,
    boundary: ({ label, width, height }: Node) => {
      const base = box({ label, width, height, fill: "#f1f5f9", stroke: "#64748b" });
      return base;
    },
  },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
