// GitHub pull request review, based on https://docs.github.com/en/pull-requests/get-started/reviewing-pull-requests-quickstart
// It follows a reviewer from opening a pull request through reviewing its diff,
// saving line comments as pending, then submitting a review decision.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "pull-request", class: "box", x: 40, y: 70, label: "Open pull request" },
    { id: "files-changed", class: "box", x: 260, y: 70, label: "Files changed\nReview diff" },
    { id: "line-comment", class: "box", x: 490, y: 70, label: "Comment on line\n(optional suggestion)" },
    { id: "pending-review", class: "box", x: 730, y: 70, label: "Start a review\nComment saved pending" },
    { id: "submit-review", class: "box", x: 980, y: 70, label: "Submit review\nAdd summary" },
    { id: "decision", class: "box", x: 1210, y: 25, label: "Choose decision\nComment / Approve /\nRequest changes" },
    { id: "timeline", class: "box", x: 1210, y: 150, label: "Submitted review\nComments in PR timeline" },
  ],
  edges: [
    { from: "pull-request", to: "files-changed", label: "select tab" },
    { from: "files-changed", to: "line-comment", label: "choose changed line" },
    { from: "line-comment", to: "pending-review", label: "Start a review" },
    { from: "pending-review", to: "submit-review", label: "finish review" },
    { from: "submit-review", to: "decision", label: "select decision" },
    { from: "decision", to: "timeline", label: "Submit review" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
