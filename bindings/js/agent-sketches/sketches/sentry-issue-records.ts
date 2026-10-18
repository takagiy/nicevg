// Sentry, based on https://github.com/getsentry/sentry/blob/master/src/sentry/models/organization.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/project.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/group.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/event.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/grouphash.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/groupenvironment.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/groupassignee.py,
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/grouphistory.py, and
// https://github.com/getsentry/sentry/blob/master/src/sentry/models/release.py.
// Shows the records behind an error issue: projects scope event groups, hashes associate incoming
// event fingerprints with groups, and environments, assignment, release and activity add context.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; width?: number; height?: number };

const nodes: Node[] = [
  { id: "organization", class: "box", x: 60, y: 40, label: "Organization\nsentry_organization\nid (PK) · slug · name", width: 230 },
  { id: "project", class: "box", x: 360, y: 40, label: "Project\nsentry_project\nid (PK) · organization_id (FK)\nslug · name", width: 250 },
  { id: "group", class: "box", x: 700, y: 40, label: "Group (issue)\nsentry_group\nid (PK) · project_id (FK)\nshort_id · message · status", width: 260 },
  { id: "event", class: "box", x: 1040, y: 40, label: "Event\nEventStore record\nid · project_id · group_id\ntimestamp · event data", width: 260 },
  { id: "grouphash", class: "box", x: 700, y: 250, label: "GroupHash\nsentry_grouphash\nid (PK) · project_id · group_id\nhash · latest", width: 260 },
  { id: "groupenvironment", class: "box", x: 360, y: 250, label: "GroupEnvironment\nsentry_groupenvironment\ngroup_id · environment_id\nfirst_seen · last_seen · count", width: 250 },
  { id: "environment", class: "box", x: 60, y: 250, label: "Environment\nsentry_environment\nid (PK) · project_id (FK)\nname", width: 230 },
  { id: "assignee", class: "box", x: 1040, y: 250, label: "GroupAssignee\nsentry_groupassignee\ngroup_id · project_id\nassigned_to_type · assigned_to_id", width: 270 },
  { id: "user", class: "box", x: 1360, y: 250, label: "User\nauth user record\nid · username", width: 220 },
  { id: "team", class: "box", x: 1360, y: 410, label: "Team\nsentry_team\nid · organization_id · slug", width: 220 },
  { id: "release", class: "box", x: 1040, y: 440, label: "Release\nsentry_release\nid (PK) · project_id (FK)\nversion · date_released", width: 270 },
  { id: "grouphistory", class: "box", x: 700, y: 440, label: "GroupHistory\nsentry_grouphistory\nid (PK) · group_id (FK) · project_id\nactor_id · type · datetime · data", width: 280 },
];

const sketch = {
  nodes,
  edges: [
    { from: "project", to: "organization", label: "organization_id" },
    { from: "group", to: "project", label: "project_id" },
    { from: "event", to: "group", label: "group_id (aggregation)" },
    { from: "event", to: "project", label: "project_id" },
    { from: "grouphash", to: "group", label: "group_id" },
    { from: "grouphash", to: "project", label: "project_id" },
    { from: "groupenvironment", to: "group", label: "group_id" },
    { from: "groupenvironment", to: "environment", label: "environment_id" },
    { from: "environment", to: "project", label: "project_id" },
    { from: "assignee", to: "group", label: "group_id" },
    { from: "assignee", to: "user", label: "assigned_to_id (user type)" },
    { from: "assignee", to: "team", label: "assigned_to_id (team type)" },
    { from: "team", to: "organization", label: "organization_id" },
    { from: "release", to: "project", label: "project_id" },
    { from: "grouphistory", to: "group", label: "group_id" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
