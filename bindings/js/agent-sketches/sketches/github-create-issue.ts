// GitHub Issues, based on https://docs.github.com/en/issues/tracking-your-work-with-issues/learning-about-issues/quickstart,
// https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-an-issue, and
// https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms.
// It follows a contributor creating a bug issue in a repository: repository access and issue settings,
// the new issue form (including optional metadata), submission, and the resulting issue URL.

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
    { id: "contributor", class: "box", x: 30, y: 170, label: "Contributor\nread access is enough" },
    { id: "repository", class: "box", x: 270, y: 170, label: "Repository\nIssues enabled" },
    { id: "issues-tab", class: "box", x: 500, y: 170, label: "Issues tab\nNew issue" },
    { id: "template-chooser", class: "box", x: 730, y: 170, label: "Template chooser\nchoose bug form or blank" },
    { id: "issue-form", class: "box", x: 990, y: 170, label: "New issue form\ntitle + Markdown body" },
    { id: "metadata", class: "box", x: 990, y: 360, label: "Optional metadata\nassignee · labels · milestone\nproject · issue type" },
    { id: "submit", class: "box", x: 730, y: 360, label: "Submit new issue" },
    { id: "issue", class: "box", x: 470, y: 360, label: "Created issue\nunique URL + issue number" },
    { id: "labels", class: "box", x: 270, y: 360, label: "Labels\nform defaults apply\nif labels exist in repository" },
    { id: "assignee", class: "box", x: 30, y: 360, label: "Assignee / milestone / project\nselected values attached" },
    { id: "issue-page", class: "box", x: 470, y: 550, label: "Issue page\nedit fields or add comments" },
    { id: "collaborators", class: "box", x: 730, y: 550, label: "Collaborators\nfollow up in issue conversation" },
  ],
  edges: [
    { from: "contributor", to: "repository", label: "open repository" },
    { from: "repository", to: "issues-tab", label: "navigate to Issues" },
    { from: "issues-tab", to: "template-chooser", label: "New issue" },
    { from: "template-chooser", to: "issue-form", label: "select form; or open blank issue" },
    { from: "issue-form", to: "metadata", label: "fill title and description" },
    { from: "metadata", to: "submit", label: "optional fields" },
    { from: "issue-form", to: "submit", label: "Submit new issue" },
    { from: "submit", to: "issue", label: "create issue" },
    { from: "issue", to: "labels", label: "template default labels" },
    { from: "issue", to: "assignee", label: "selected / default metadata" },
    { from: "issue", to: "issue-page", label: "open unique issue URL" },
    { from: "issue-page", to: "collaborators", label: "share URL; add comments or @mentions" },
    { from: "collaborators", to: "issue-page", label: "continue conversation" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
