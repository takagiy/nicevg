// Git documentation: https://git-scm.com/docs/git-commit,
// https://git-scm.com/docs/githooks, and https://git-scm.com/book/en/v2/Git-Internals-Git-Objects.
// This sketch follows an ordinary `git commit`: hooks and message preparation run first, then
// the staged index is written as a tree and commit object, the current ref advances, and post-commit runs.
import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

type GitStep = SketchNode & { class: "box"; label: string };

const sketch = {
  nodes: [
    { id: "user", class: "box", x: 20, y: 205, label: "User\ngit commit" },
    { id: "git_commit", class: "box", x: 200, y: 205, label: "git-commit\ncommand" },
    { id: "pre_commit", class: "box", x: 400, y: 35, label: "pre-commit\nhook" },
    { id: "prepare_msg", class: "box", x: 400, y: 150, label: "prepare-commit-msg\nhook" },
    { id: "message_file", class: "box", x: 620, y: 150, label: "$GIT_DIR/\nCOMMIT_EDITMSG" },
    { id: "editor", class: "box", x: 820, y: 150, label: "Editor\nuser writes message" },
    { id: "commit_msg", class: "box", x: 1020, y: 150, label: "commit-msg\nhook" },
    { id: "index", class: "box", x: 400, y: 355, label: "$GIT_DIR/index\nstaged entries" },
    { id: "tree", class: "box", x: 620, y: 355, label: "Tree object\nsnapshot of index" },
    { id: "commit_object", class: "box", x: 820, y: 355, label: "Commit object\ntree + parent + identities\n+ timestamp + message" },
    { id: "objects", class: "box", x: 1020, y: 355, label: "$GIT_DIR/objects\nobject database" },
    { id: "head_ref", class: "box", x: 1220, y: 355, label: "Current branch ref\npoints to new commit" },
    { id: "post_commit", class: "box", x: 1420, y: 355, label: "post-commit\nhook" },
    { id: "abort", class: "box", x: 820, y: 35, label: "Commit aborted\nnon-zero hook status" },
  ],
  edges: [
    { from: "user", to: "git_commit", label: "invoke" },
    { from: "git_commit", to: "pre_commit", label: "run before message" },
    { from: "pre_commit", to: "prepare_msg", label: "success" },
    { from: "git_commit", to: "index", label: "read staged state" },
    { from: "prepare_msg", to: "message_file", label: "prepare default message" },
    { from: "message_file", to: "editor", label: "edit message" },
    { from: "editor", to: "message_file", label: "save text" },
    { from: "message_file", to: "commit_msg", label: "message file path" },
    { from: "commit_msg", to: "index", label: "success; continue" },
    { from: "index", to: "tree", label: "write-tree: staged paths + blob OIDs" },
    { from: "tree", to: "commit_object", label: "tree OID" },
    { from: "commit_msg", to: "commit_object", label: "message" },
    { from: "commit_object", to: "objects", label: "write commit object" },
    { from: "objects", to: "head_ref", label: "new commit OID" },
    { from: "head_ref", to: "post_commit", label: "ref updated" },
    { from: "pre_commit", to: "abort", label: "non-zero; no commit" },
    { from: "prepare_msg", to: "abort", label: "non-zero; no commit" },
    { from: "commit_msg", to: "abort", label: "non-zero; no commit" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<GitStep>;

export default sketch;
