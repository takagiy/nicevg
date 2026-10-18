// GitHub pull request creation, based on https://docs.github.com/en/pull-requests/how-tos/create-pull-requests/creating-a-pull-request
// It follows a contributor's pushed branch through GitHub's compare page,
// branch selection and pull request form to the newly opened pull request.

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
    { id: "branch", class: "box", x: 30, y: 170, label: "Repository branch\ncontains proposed commits" },
    { id: "compare-link", class: "box", x: 290, y: 170, label: "Compare & pull request\nopen for this branch" },
    { id: "branches", class: "box", x: 570, y: 170, label: "Open a pull request\nchoose base + compare branch" },
    { id: "diff", class: "box", x: 870, y: 170, label: "Compare changes\nbase...head diff" },
    { id: "form", class: "box", x: 870, y: 390, label: "Pull request form\ntitle + description" },
    { id: "reviewers", class: "box", x: 570, y: 390, label: "Optional reviewers\nassign people or team" },
    { id: "create", class: "box", x: 290, y: 390, label: "Create Pull Request\n(or Create Draft Pull Request)" },
    { id: "pull-request", class: "box", x: 30, y: 390, label: "Open pull request\nproposes merging head into base" },
  ],
  edges: [
    { from: "branch", to: "compare-link", label: "navigate to repository branch" },
    { from: "compare-link", to: "branches", label: "Compare & pull request" },
    { from: "branches", to: "diff", label: "select base and compare branches" },
    { from: "diff", to: "form", label: "review proposed changes" },
    { from: "form", to: "reviewers", label: "enter title and description" },
    { from: "reviewers", to: "create", label: "optional reviewer selection" },
    { from: "create", to: "pull-request", label: "submit" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
