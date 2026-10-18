// Apache DataFusion query planning and execution, based on its architecture,
// SQL, DataFrame, optimizer, and table-provider documentation. Shows how SQL or
// DataFrame operations become optimized plans, then streams of Arrow batches.
// https://datafusion.apache.org/library-user-guide/architecture.html
// https://datafusion.apache.org/user-guide/sql/index.html
// https://datafusion.apache.org/user-guide/dataframe.html
// https://datafusion.apache.org/library-user-guide/query-optimizer.html
// https://datafusion.apache.org/library-user-guide/custom-table-providers.html
// https://github.com/apache/datafusion/blob/main/datafusion/core/src/physical_planner.rs

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

const nodes: Node[] = [
  { id: "query", class: "box", x: 25, y: 180, label: "SQL statement\nSELECT / EXPLAIN", width: 190, height: 65, fill: "#fef3c7" },
  { id: "context", class: "box", x: 255, y: 130, label: "SessionContext\nSQL + DataFrame entry points", width: 230, height: 72 },
  { id: "sqlplanner", class: "box", x: 535, y: 45, label: "SqlToRel\nSQL AST → LogicalPlan", width: 210, height: 66 },
  { id: "df", class: "box", x: 535, y: 245, label: "DataFrame / LogicalPlanBuilder\nselect · filter · aggregate", width: 235, height: 72 },
  { id: "logical", class: "box", x: 805, y: 145, label: "LogicalPlan\nTableScan · Filter · Projection · Aggregate", width: 270, height: 78, fill: "#dbeafe" },
  { id: "analyzer", class: "box", x: 805, y: 30, label: "Analyzer\nresolve names and types", width: 205, height: 65 },
  { id: "logicalopt", class: "box", x: 1110, y: 130, label: "Optimizer::optimize\nOptimizerRules", width: 220, height: 68 },
  { id: "provider", class: "box", x: 1100, y: 310, label: "TableProvider\nregistered table / catalog", width: 225, height: 68, fill: "#fce7f3" },
  { id: "scan", class: "box", x: 1390, y: 270, label: "TableProvider::scan()\nprojection · filters · limit", width: 235, height: 72 },
  { id: "physical", class: "box", x: 1390, y: 110, label: "SessionState::create_physical_plan\nPhysicalPlanner", width: 275, height: 76 },
  { id: "physicalopt", class: "box", x: 1730, y: 130, label: "PhysicalOptimizerRule\nphysical plan rewrites", width: 235, height: 70 },
  { id: "execplan", class: "box", x: 2025, y: 170, label: "ExecutionPlan tree\nFilterExec · ProjectionExec\nDataSourceExec", width: 250, height: 85, fill: "#dcfce7" },
  { id: "execute", class: "box", x: 2020, y: 340, label: "ExecutionPlan::execute(partition)\nper-partition execution", width: 255, height: 72 },
  { id: "stream", class: "box", x: 1715, y: 385, label: "SendableRecordBatchStream\nasync stream of RecordBatch", width: 265, height: 75, fill: "#e0e7ff" },
  { id: "source", class: "box", x: 1390, y: 465, label: "FileSource / external data source\nscan files or source partitions", width: 270, height: 76, fill: "#fef3c7" },
  { id: "collect", class: "box", x: 1070, y: 490, label: "DataFrame::collect()\nterminal action", width: 220, height: 68 },
  { id: "results", class: "box", x: 770, y: 490, label: "Vec<RecordBatch>\nArrow in-memory result", width: 225, height: 72, fill: "#dbeafe" },
];

const edges = [
  { from: "query", to: "context", label: "SQL text" },
  { from: "context", to: "sqlplanner", label: "SQL planning" },
  { from: "context", to: "df", label: "DataFrame API calls" },
  { from: "sqlplanner", to: "logical", label: "relational operators" },
  { from: "df", to: "logical", label: "build LogicalPlan directly" },
  { from: "logical", to: "analyzer", label: "unresolved plan" },
  { from: "analyzer", to: "logicalopt", label: "resolved LogicalPlan" },
  { from: "logicalopt", to: "logical", label: "rewrite rules; equivalent plan" },
  { from: "logicalopt", to: "physical", label: "optimized LogicalPlan" },
  { from: "logical", to: "provider", label: "TableScan resolves table source" },
  { from: "physical", to: "scan", label: "projection, pushed filters, limit" },
  { from: "scan", to: "provider", label: "provider-specific scan plan" },
  { from: "scan", to: "physical", label: "ExecutionPlan for source" },
  { from: "physical", to: "physicalopt", label: "ExecutionPlan tree" },
  { from: "physicalopt", to: "execplan", label: "optimized physical operators" },
  { from: "execplan", to: "execute", label: "execute each partition" },
  { from: "execute", to: "source", label: "open source partition / read data" },
  { from: "source", to: "stream", label: "RecordBatch values" },
  { from: "execute", to: "stream", label: "SendableRecordBatchStream" },
  { from: "collect", to: "execplan", label: "create plan, execute partitions" },
  { from: "collect", to: "stream", label: "poll streams to completion" },
  { from: "stream", to: "results", label: "RecordBatch items" },
  { from: "results", to: "context", label: "collected query result" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
