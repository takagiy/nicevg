import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

// GitLab CI/CD runner job scheduling and stuck-job recovery.
// Sources: https://docs.gitlab.com/development/cicd/ and https://docs.gitlab.com/ci/yaml/ and https://docs.gitlab.com/ci/jobs/job_troubleshooting/ and https://docs.gitlab.com/ci/runners/configure_runners/.
// Follows one pipeline job from creation through runner selection and execution, then shows the
// distinct timeout paths, automatic retry, and the project-owner fixes for jobs no runner can take.

type Kind = "step" | "decision" | "person" | "state" | "system";
interface Node extends SketchNode {
  class: Kind;
  label: string;
}

const shape = (kind: Kind) => ({ label }: Node) => {
  const lines = text(label, 12);
  const w = Math.max(112, lines.width + 24);
  const h = Math.max(48, lines.height + 18);
  const x = kind === "person" ? 18 : 0;
  const y = 0;
  const body = kind === "decision"
    ? svg`<path data-bounds="true" d="M ${w / 2} 0 L ${w} ${h / 2} L ${w / 2} ${h} L 0 ${h / 2} Z" fill="#fff7d6" stroke="#805b00"/>`
    : kind === "person"
      ? svg`<rect data-bounds="true" x="${x}" y="${y}" width="${w - 18}" height="${h}" rx="18" fill="#fce7f3" stroke="#9d174d"/>`
      : kind === "state"
        ? svg`<rect data-bounds="true" width="${w}" height="${h}" rx="22" fill="#e0f2fe" stroke="#0369a1"/>`
        : kind === "system"
          ? svg`<path data-bounds="true" d="M 12 0 H ${w} V ${h} H 0 V 12 Z" fill="#ede9fe" stroke="#5b21b6"/>`
          : svg`<rect data-bounds="true" width="${w}" height="${h}" rx="4" fill="#ecfdf5" stroke="#166534"/>`;
  return svg`${body}${lines.at({ cx: w / 2, cy: h / 2 })}`;
};

const nodes: Node[] = [
  { id: "push", class: "person", x: 35, y: 35, label: "Developer pushes\ncommit" },
  { id: "pipeline_create", class: "step", x: 220, y: 42, label: "Create pipeline" },
  { id: "yaml", class: "system", x: 410, y: 35, label: ".gitlab-ci.yml\nworkflow and jobs" },
  { id: "workflow_rules", class: "decision", x: 620, y: 35, label: "Workflow rules\nallow pipeline?" },
  { id: "pipeline_rejected", class: "state", x: 835, y: 15, label: "Pipeline not\ncreated" },
  { id: "expand_config", class: "step", x: 840, y: 110, label: "Resolve includes,\nextends, variables" },
  { id: "job_rules", class: "decision", x: 1050, y: 110, label: "Job rules match\nthis pipeline?" },
  { id: "job_omitted", class: "state", x: 1260, y: 40, label: "Job omitted\nfrom pipeline" },
  { id: "create_all_jobs", class: "step", x: 1260, y: 145, label: "Create jobs for\nall stages" },
  { id: "created", class: "state", x: 1450, y: 145, label: "Job: created" },
  { id: "first_stage", class: "decision", x: 1640, y: 145, label: "First stage?" },
  { id: "manual_gate", class: "decision", x: 1640, y: 250, label: "Manual job\ntriggered?" },
  { id: "manual_user", class: "person", x: 1850, y: 245, label: "Authorized user\nselects Play" },
  { id: "previous_stage", class: "decision", x: 1640, y: 365, label: "All prior-stage\njobs succeeded?" },
  { id: "needs_gate", class: "decision", x: 1640, y: 480, label: "All needs:\ndependencies done?" },
  { id: "drop_check", class: "system", x: 1875, y: 480, label: "DropNotRunnableBuildsService" },
  { id: "quota", class: "decision", x: 2080, y: 480, label: "CI/CD minutes\navailable?" },
  { id: "quota_fail", class: "state", x: 2295, y: 440, label: "failed:\nci_quota_exceeded" },
  { id: "pending", class: "state", x: 2080, y: 610, label: "Job: pending" },
  { id: "runner_loop", class: "step", x: 1850, y: 735, label: "Runner worker\nrequests next job" },
  { id: "job_request", class: "system", x: 1640, y: 735, label: "POST /api/v4/jobs/request\nrunner_token" },
  { id: "workhorse", class: "system", x: 1420, y: 735, label: "GitLab Workhorse\nlong-poll entry" },
  { id: "last_update", class: "step", x: 1200, y: 735, label: "Read runner token +\nX-GitLab-Last-Update" },
  { id: "subscribe", class: "system", x: 980, y: 735, label: "Subscribe Redis PubSub\nrunner key" },
  { id: "changed", class: "decision", x: 760, y: 735, label: "last_update\nchanged?" },
  { id: "hold_poll", class: "step", x: 535, y: 835, label: "Hold request up to\napiCiLongPollingDuration" },
  { id: "tick_job", class: "system", x: 300, y: 835, label: "Sidekiq tick updates\neligible runner keys" },
  { id: "rails_api", class: "system", x: 980, y: 600, label: "Rails runner API" },
  { id: "register_service", class: "system", x: 760, y: 600, label: "Ci::RegisterJobService" },
  { id: "queue", class: "system", x: 535, y: 600, label: "Pending jobs pool" },
  { id: "scope_select", class: "step", x: 300, y: 600, label: "Query jobs by\nrunner registration scope" },
  { id: "instance_runner", class: "decision", x: 70, y: 600, label: "Instance runner?" },
  { id: "fair_schedule", class: "step", x: 70, y: 735, label: "Fair scheduling:\nprojects with fewer\nrunning builds first" },
  { id: "group_runner", class: "decision", x: 70, y: 875, label: "Group runner?" },
  { id: "group_jobs", class: "step", x: 300, y: 875, label: "Select jobs visible\nto registered group" },
  { id: "project_runner", class: "decision", x: 535, y: 875, label: "Project runner?" },
  { id: "project_jobs", class: "step", x: 760, y: 875, label: "Select jobs visible\nto registered project" },
  { id: "tag_match", class: "decision", x: 980, y: 1000, label: "Runner has every\njob tag?" },
  { id: "untagged", class: "decision", x: 1200, y: 1000, label: "Untagged job and\nrunner accepts untagged?" },
  { id: "protected", class: "decision", x: 1420, y: 1000, label: "Protected runner\nand protected ref?" },
  { id: "additional_policy", class: "step", x: 1640, y: 1000, label: "Check remaining\nrunner policies" },
  { id: "candidate", class: "step", x: 1850, y: 1000, label: "Try first eligible\npending job" },
  { id: "claim", class: "decision", x: 2080, y: 1000, label: "Atomic assignment\nconflict?" },
  { id: "next_candidate", class: "step", x: 2295, y: 1000, label: "Rescue conflict;\ntry next job" },
  { id: "assigned", class: "state", x: 2080, y: 1130, label: "Job assigned;\n201 job payload" },
  { id: "runner_executor", class: "system", x: 1850, y: 1240, label: "GitLab Runner\nexecutor" },
  { id: "prepare", class: "step", x: 1640, y: 1240, label: "Prepare build\ndirectory and executor" },
  { id: "clone", class: "step", x: 1420, y: 1240, label: "Clone sources\nwith job_token" },
  { id: "artifacts", class: "step", x: 1200, y: 1240, label: "Download dependency\nartifacts" },
  { id: "script", class: "step", x: 980, y: 1240, label: "Run before_script,\nscript, after_script" },
  { id: "upload", class: "step", x: 760, y: 1240, label: "Upload artifacts\nand job trace" },
  { id: "report", class: "step", x: 535, y: 1240, label: "POST job status\nand trace updates" },
  { id: "success", class: "state", x: 300, y: 1240, label: "Job succeeded" },
  { id: "failure_reason", class: "decision", x: 300, y: 1390, label: "Classify failure_reason" },
  { id: "retry_config", class: "system", x: 535, y: 1390, label: "retry:max and\nretry:when in YAML" },
  { id: "retry_match", class: "decision", x: 760, y: 1390, label: "Reason matches and\nretry count remains?" },
  { id: "retry_new_job", class: "step", x: 980, y: 1510, label: "Create retry attempt\n(re-evaluate pipeline)" },
  { id: "retry_pending", class: "state", x: 1200, y: 1510, label: "New attempt\nreturns to pending" },
  { id: "final_failed", class: "state", x: 980, y: 1390, label: "Job failed;\nno retry" },
  { id: "human_inspect", class: "person", x: 1200, y: 1390, label: "Developer inspects\njob log and reason" },
  { id: "fix_yaml", class: "person", x: 1420, y: 1510, label: "Fix script, image,\ntags or runner config" },
  { id: "manual_retry", class: "person", x: 1640, y: 1510, label: "Authorized user\nselects Retry" },
  { id: "pending_timer", class: "system", x: 1420, y: 600, label: "DropPendingService\nchecks age and match" },
  { id: "matching_exists", class: "decision", x: 1640, y: 600, label: "Any runner matches\njob requirements?" },
  { id: "no_match_1h", class: "step", x: 1850, y: 365, label: "No match: pending\nfor 1 hour" },
  { id: "match_24h", class: "step", x: 2080, y: 325, label: "Matching runner exists,\nnot picked up in 24h" },
  { id: "stuck_failed", class: "state", x: 2295, y: 325, label: "failed: stuck pending\nreason recorded" },
  { id: "stuck_retry", class: "decision", x: 2295, y: 600, label: "retry:when includes\nstuck pending reason?" },
  { id: "stuck_requeue", class: "step", x: 2080, y: 735, label: "Retry creates attempt;\nrunner selection repeats" },
  { id: "owner", class: "person", x: 1850, y: 875, label: "Project owner\nchecks runner setup" },
  { id: "enable_runner", class: "step", x: 1640, y: 875, label: "Enable eligible\nproject/group/instance runner" },
  { id: "correct_tags", class: "step", x: 1420, y: 875, label: "Add all required tags;\nallow untagged if needed" },
  { id: "protected_fix", class: "step", x: 1200, y: 875, label: "Use protected ref or\nchange runner protection" },
  { id: "quota_owner", class: "person", x: 980, y: 480, label: "Project owner\nadds minutes / waits\nfor monthly reset" },
  { id: "system_failure", class: "decision", x: 70, y: 1390, label: "Runner interrupted,\nexternal dependency,\nor system failure?" },
  { id: "runner_restart", class: "step", x: 70, y: 1510, label: "Runner restarts or\nrecovers dependency" },
  { id: "retry_reason_set", class: "system", x: 300, y: 1510, label: "runner_interrupted /\nrunner_system_failure /\nrunner_external_dependency_failure" },
  { id: "running_timeout", class: "system", x: 70, y: 1240, label: "Server timeout: configured\ntimeout + 15 minutes" },
  { id: "no_updates", class: "system", x: 70, y: 1080, label: "No running updates\nfor 30 minutes" },
  { id: "no_updates_reason", class: "state", x: 300, y: 1080, label: "no_updates_running\nor server_timeout_running" },
  { id: "retry_reason_list", class: "step", x: 535, y: 1080, label: "retry:when can include\nstuck_pending_*, no_updates_running,\nrunner failures, script_failure" },
  { id: "stuck_retry_reasons", class: "step", x: 760, y: 600, label: "Specific reasons:\nstuck_pending_with_matching_runners\nstuck_pending_no_matching_runners" },
];

const edges = [
  ["push", "pipeline_create"], ["pipeline_create", "yaml"], ["yaml", "workflow_rules"], ["workflow_rules", "pipeline_rejected", "no"], ["workflow_rules", "expand_config", "yes"], ["expand_config", "job_rules"], ["job_rules", "job_omitted", "no"], ["job_rules", "create_all_jobs", "yes"], ["create_all_jobs", "created"], ["created", "first_stage"], ["first_stage", "pending", "yes"], ["first_stage", "manual_gate", "no"], ["manual_gate", "manual_user", "manual job"], ["manual_user", "pending", "Play"], ["manual_gate", "previous_stage", "not manual"], ["previous_stage", "pending", "all succeeded"], ["previous_stage", "needs_gate", "needs dependencies"], ["needs_gate", "pending", "all complete"], ["needs_gate", "drop_check", "not yet runnable"], ["drop_check", "quota"], ["quota", "quota_fail", "no"], ["quota", "pending", "yes"], ["quota_fail", "quota_owner"],
  ["runner_loop", "job_request"], ["job_request", "workhorse", "runner_token"], ["workhorse", "last_update"], ["last_update", "subscribe"], ["subscribe", "changed"], ["changed", "rails_api", "changed / no key"], ["changed", "hold_poll", "unchanged"], ["hold_poll", "tick_job"], ["tick_job", "subscribe", "publish key change"], ["rails_api", "register_service"], ["register_service", "queue"], ["queue", "scope_select"], ["scope_select", "instance_runner"], ["instance_runner", "fair_schedule", "yes"], ["fair_schedule", "tag_match"], ["instance_runner", "group_runner", "no"], ["group_runner", "group_jobs", "yes"], ["group_jobs", "tag_match"], ["group_runner", "project_runner", "no"], ["project_runner", "project_jobs", "yes"], ["project_jobs", "tag_match"], ["tag_match", "untagged", "job has no tags"], ["tag_match", "protected", "all tags match"], ["untagged", "protected", "accepted"], ["untagged", "pending_timer", "filtered out"], ["protected", "additional_policy", "allowed"], ["protected", "pending_timer", "filtered out"], ["additional_policy", "candidate"], ["candidate", "claim"], ["claim", "next_candidate", "conflict"], ["next_candidate", "candidate"], ["claim", "assigned", "claimed"], ["assigned", "runner_executor"],
  ["runner_executor", "prepare"], ["prepare", "clone"], ["clone", "artifacts"], ["artifacts", "script"], ["script", "upload"], ["upload", "report"], ["report", "success", "success"], ["report", "failure_reason", "failure"], ["success", "previous_stage", "unlocks next stage"], ["failure_reason", "system_failure"], ["system_failure", "runner_restart", "yes"], ["runner_restart", "retry_reason_set"], ["retry_reason_set", "retry_config"], ["system_failure", "running_timeout", "timeout"], ["running_timeout", "no_updates"], ["no_updates", "no_updates_reason"], ["no_updates_reason", "retry_reason_list"], ["retry_reason_list", "retry_config"], ["failure_reason", "retry_config"], ["retry_config", "retry_match"], ["retry_match", "retry_new_job", "matches / count remains"], ["retry_new_job", "retry_pending"], ["retry_pending", "pending"], ["retry_match", "final_failed", "no"], ["final_failed", "human_inspect"], ["human_inspect", "fix_yaml"], ["fix_yaml", "manual_retry"], ["manual_retry", "retry_new_job"],
  ["pending", "runner_loop", "poll loop"], ["pending", "pending_timer", "age watch"], ["pending_timer", "matching_exists"], ["matching_exists", "no_match_1h", "none"], ["matching_exists", "match_24h", "exists"], ["no_match_1h", "stuck_failed"], ["match_24h", "stuck_failed"], ["stuck_failed", "stuck_retry"], ["stuck_retry", "stuck_requeue", "configured retry"], ["stuck_requeue", "pending"], ["stuck_retry", "owner", "no retry"], ["owner", "enable_runner"], ["owner", "correct_tags"], ["owner", "protected_fix"], ["enable_runner", "pending"], ["correct_tags", "pending"], ["protected_fix", "pending"], ["stuck_retry", "stuck_retry_reasons"], ["stuck_retry_reasons", "retry_config"],
];

const sketch = {
  nodes,
  edges: edges.map(([from, to, label]) => ({ from: from!, to: to!, ...(label ? { label } : {}) })),
  style: { step: shape("step"), decision: shape("decision"), person: shape("person"), state: shape("state"), system: shape("system") },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
