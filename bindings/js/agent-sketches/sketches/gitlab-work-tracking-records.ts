// GitLab, based on https://docs.gitlab.com/development/database/database_dictionary/,
// https://docs.gitlab.com/development/database/foreign_keys/, https://docs.gitlab.com/api/issues/,
// https://docs.gitlab.com/api/merge_requests/, and https://docs.gitlab.com/user/project/milestones/.
// Shows the project work-tracking records: issues and merge requests, the people and metadata
// attached to them, and how merge requests can be associated with the issues they close.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes = [
  { id: "namespaces", class: "box", x: 40, y: 60, label: "namespaces\nid · name · parent_id" },
  { id: "projects", class: "box", x: 340, y: 60, label: "projects\nid · namespace_id · name" },
  { id: "users", class: "box", x: 40, y: 300, label: "users\nid · username · name" },
  { id: "issues", class: "box", x: 360, y: 280, label: "issues\nid · project_id · iid · title\nauthor_id · milestone_id · state" },
  { id: "issue_assignees", class: "box", x: 40, y: 500, label: "issue_assignees\nissue_id · user_id" },
  { id: "milestones", class: "box", x: 360, y: 520, label: "milestones\nid · project_id / group_id\ntitle · due_date · state" },
  { id: "labels", class: "box", x: 700, y: 60, label: "labels\nid · project_id / group_id\ntitle · color" },
  { id: "label_links", class: "box", x: 700, y: 290, label: "label_links\nlabel_id · target_type · target_id" },
  { id: "notes", class: "box", x: 700, y: 520, label: "notes\nid · project_id · noteable_type\nnoteable_id · author_id · note" },
  { id: "merge_requests", class: "box", x: 1050, y: 60, label: "merge_requests\nid · target_project_id · iid\nsource_project_id · author_id\nsource_branch · target_branch · state" },
  { id: "merge_request_diffs", class: "box", x: 1050, y: 310, label: "merge_request_diffs\nid · merge_request_id · head_commit_sha" },
  { id: "merge_requests_closing_issues", class: "box", x: 1050, y: 540, label: "merge_requests_closing_issues\nmerge_request_id · issue_id" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "projects", to: "namespaces", label: "namespace_id" },
    { from: "namespaces", to: "namespaces", label: "parent_id (nested group)" },
    { from: "issues", to: "projects", label: "project_id" },
    { from: "issues", to: "users", label: "author_id" },
    { from: "issues", to: "milestones", label: "milestone_id (optional)" },
    { from: "issue_assignees", to: "issues", label: "issue_id" },
    { from: "issue_assignees", to: "users", label: "user_id; many-to-many assignees" },
    { from: "milestones", to: "projects", label: "project_id (project milestone)" },
    { from: "milestones", to: "namespaces", label: "group_id (group milestone)" },
    { from: "labels", to: "projects", label: "project_id (project label)" },
    { from: "labels", to: "namespaces", label: "group_id (group label)" },
    { from: "label_links", to: "labels", label: "label_id" },
    { from: "label_links", to: "issues", label: "target_type + target_id when issue" },
    { from: "label_links", to: "merge_requests", label: "target_type + target_id when MR" },
    { from: "notes", to: "projects", label: "project_id" },
    { from: "notes", to: "users", label: "author_id" },
    { from: "notes", to: "issues", label: "noteable_type + noteable_id when issue" },
    { from: "notes", to: "merge_requests", label: "noteable_type + noteable_id when MR" },
    { from: "merge_requests", to: "projects", label: "target_project_id; project_id is target" },
    { from: "merge_requests", to: "projects", label: "source_project_id (may be a fork)" },
    { from: "merge_requests", to: "users", label: "author_id" },
    { from: "merge_request_diffs", to: "merge_requests", label: "merge_request_id; revisions" },
    { from: "merge_requests_closing_issues", to: "merge_requests", label: "merge_request_id" },
    { from: "merge_requests_closing_issues", to: "issues", label: "issue_id; closes / relates" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
