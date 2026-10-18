// Temporal Workflow Execution lifecycle, based on:
// https://docs.temporal.io/encyclopedia/event-history
// https://docs.temporal.io/tasks
// https://docs.temporal.io/temporal-service/persistence
// https://docs.temporal.io/workflows
// The sketch follows an execution from start through replayed Workflow Tasks and Activity attempts
// to a terminal outcome, including signal-driven progress and retry after a failed attempt.
import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = SketchNode & { class: "box"; label: string; width?: number; height?: number };

const nodes = [
  { id: "client", class: "box", x: 25, y: 170, label: "Temporal Client\nStart / Signal / Query" },
  { id: "frontend", class: "box", x: 300, y: 45, label: "Frontend Service\nAPI 要求を受け付ける" },
  { id: "execution", class: "box", x: 600, y: 170, label: "Workflow Execution\nRunning → Completed / Failed\nCanceled / Terminated / Timed Out" },
  { id: "history", class: "box", x: 950, y: 40, label: "History Service\nイベントを追記し状態を更新" },
  { id: "persistence", class: "box", x: 1260, y: 145, label: "Persistence DB\nExecution + append-only History" },
  { id: "matching", class: "box", x: 940, y: 370, label: "Matching Service\nTask Queue の配送" },
  { id: "task-queue", class: "box", x: 1250, y: 430, label: "Task Queue\nWorkflow / Activity Tasks" },
  { id: "workflow-worker", class: "box", x: 1540, y: 260, label: "Workflow Worker\n履歴を replay して Command を生成" },
  { id: "activity-worker", class: "box", x: 1540, y: 520, label: "Activity Worker\nActivity を実行" },
] satisfies Node[];

const edges = [
  { from: "client", to: "frontend", label: "StartWorkflowExecution\nWorkflowId・TaskQueue・引数" },
  { from: "frontend", to: "history", label: "開始要求を転送" },
  { from: "history", to: "persistence", label: "WorkflowExecutionStarted\nExecution / History を保存" },
  { from: "history", to: "matching", label: "Workflow Task を作成" },
  { from: "matching", to: "task-queue", label: "Workflow Task を載せる" },
  { from: "task-queue", to: "workflow-worker", label: "poll で Workflow Task と履歴を取得" },
  { from: "workflow-worker", to: "history", label: "replay 後の Command\nActivity / Timer / 完了" },
  { from: "history", to: "persistence", label: "Command を Event に変換して追記" },
  { from: "history", to: "matching", label: "ActivityTaskScheduled を配送" },
  { from: "matching", to: "task-queue", label: "Activity Task を載せる" },
  { from: "task-queue", to: "activity-worker", label: "poll で Activity Task を取得" },
  { from: "activity-worker", to: "history", label: "結果または失敗を報告" },
  { from: "history", to: "persistence", label: "ActivityTaskCompleted / Failed を追記" },
  { from: "history", to: "matching", label: "失敗時: retry policy に従い再スケジュール" },
  { from: "client", to: "frontend", label: "SignalWorkflowExecution\nSignal 名 + Payload" },
  { from: "frontend", to: "history", label: "SignalReceived を記録" },
  { from: "history", to: "execution", label: "追記済み Event History から状態を復元" },
  { from: "workflow-worker", to: "client", label: "Query 結果 / Workflow 結果" },
] satisfies NonNullable<Sketch<Node>["edges"]>;

export default {
  nodes,
  edges,
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
