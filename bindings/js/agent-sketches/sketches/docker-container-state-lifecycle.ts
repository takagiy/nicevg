// Moby (Docker Engine), based on https://github.com/moby/moby/blob/master/daemon/container/state.go,
// https://github.com/moby/moby/blob/master/daemon/start.go, https://github.com/moby/moby/blob/master/daemon/monitor.go,
// and https://github.com/moby/moby/blob/master/daemon/delete.go. Follows the daemon's container.State
// fields from creation through start, pause, exit/restart, and removal, including failure branches.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes = [
  { id: "created", class: "box", x: 40, y: 120, label: "Created\nStartedAt is zero" },
  { id: "start", class: "box", x: 300, y: 120, label: "containerStart\nmount, network, dirs, mounts, spec\ncontainerd task.Start" },
  { id: "start_failed", class: "box", x: 300, y: 330, label: "Start error\nErrorMsg + nonzero ExitCode\nReset + Cleanup; remains Created" },
  { id: "running", class: "box", x: 590, y: 120, label: "Running\nRunning=true, Pid, StartedAt\nExitCode=0; health monitor starts" },
  { id: "paused", class: "box", x: 880, y: 30, label: "Paused\nRunning=true AND Paused=true\nfreezer suspends processes" },
  { id: "exit_event", class: "box", x: 880, y: 240, label: "containerd EventExit\nstop healthchecks; delete task\nReset; read exit code + time" },
  { id: "restart_check", class: "box", x: 1170, y: 240, label: "RestartManager.ShouldRestart\nexit code + run duration\npolicy, manual stop, daemon shutdown" },
  { id: "restarting", class: "box", x: 1450, y: 120, label: "Restarting\nRunning=true, Restarting=true\nPid=0; FinishedAt + ExitCode saved" },
  { id: "exited", class: "box", x: 1170, y: 440, label: "Exited\nRunning=false; Pid=0\nFinishedAt + ExitCode saved" },
  { id: "manual_start", class: "box", x: 590, y: 440, label: "docker start\nvalidateState rejects paused, running,\nremoving, or dead containers" },
  { id: "removing", class: "box", x: 880, y: 590, label: "RemovalInProgress=true\nrm requires stopped; force rm kills\ncontainerStop timeout: 3 seconds" },
  { id: "dead", class: "box", x: 1170, y: 590, label: "Dead=true\ncheckpoint before deleting metadata\nblocks future start / restart" },
  { id: "remove_error", class: "box", x: 1450, y: 590, label: "Removal error\nrestore RWLayer if needed\nSetRemovalError wakes waiters" },
  { id: "removed", class: "box", x: 1450, y: 440, label: "Removed\ncontainer + replica metadata deleted\nSetRemoved wakes waiters" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "created", to: "start", label: "POST /containers/{id}/start" },
    { from: "start", to: "running", label: "task.Start succeeds; SetRunning" },
    { from: "start", to: "start_failed", label: "setup / create / task start fails" },
    { from: "start_failed", to: "start", label: "correct cause; retry start" },
    { from: "running", to: "paused", label: "EventPaused sets Paused=true" },
    { from: "paused", to: "running", label: "EventResumed clears Paused" },
    { from: "running", to: "exit_event", label: "process exits; EventExit" },
    { from: "paused", to: "exit_event", label: "process exits" },
    { from: "exit_event", to: "restart_check", label: "exitCode + execDuration" },
    { from: "restart_check", to: "restarting", label: "policy says restart; SetRestarting" },
    { from: "restarting", to: "start", label: "wait elapsed; containerStart" },
    { from: "restart_check", to: "exited", label: "no restart; SetStopped" },
    { from: "exited", to: "manual_start", label: "POST /containers/{id}/start" },
    { from: "created", to: "manual_start", label: "start request" },
    { from: "manual_start", to: "start", label: "validation passes" },
    { from: "exited", to: "removing", label: "docker rm" },
    { from: "created", to: "removing", label: "docker rm; never started" },
    { from: "removing", to: "dead", label: "stop + cleanup; set Dead=true" },
    { from: "dead", to: "removed", label: "release layer; delete root + records" },
    { from: "dead", to: "remove_error", label: "layer or filesystem removal fails" },
    { from: "remove_error", to: "removed", label: "later cleanup succeeds" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
