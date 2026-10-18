// Django contributor and release process:
// https://docs.djangoproject.com/en/6.0/internals/contributing/writing-code/submitting-patches/
// https://docs.djangoproject.com/en/6.0/internals/contributing/triaging-tickets/
// https://docs.djangoproject.com/en/6.0/internals/release-process/
// https://docs.djangoproject.com/en/6.0/internals/howto-release-django/
// Follows a non-trivial code change from its accepted Trac ticket and reviewed PR into main,
// through the feature-freeze branch and staged versions, to signed artifacts published on PyPI.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

export default {
  nodes: [
    { id: "change", class: "box", x: 30, y: 150, label: "Contributor change\ncode + regression test\ndocs for behavior change" },
    { id: "ticket", class: "box", x: 260, y: 55, label: "Trac ticket\n# · component · accepted\nNeeds tests / docs flags" },
    { id: "pr", class: "box", x: 260, y: 250, label: "GitHub pull request\nbranch · commits · ticket link\nsquashed commit + formatted message" },
    { id: "review", class: "box", x: 545, y: 110, label: "Trac review state\nNeeds PR Review → Ready for checkin\nmerger final review" },
    { id: "main", class: "box", x: 815, y: 125, label: "Django main\nmerged commit\nfeatures for next A.B" },
    { id: "stable", class: "box", x: 1045, y: 265, label: "stable/A.B.x\nforked from main at feature freeze\npost-alpha fixes backported" },
    { id: "release", class: "box", x: 1290, y: 65, label: "Release entry\nversion · target date · LTS\nalpha → beta → rc → final\nis_active: false until published" },
    { id: "artifacts", class: "box", x: 1550, y: 245, label: "Django release artifacts\nVERSION tuple · signed Git tag\ntarball + wheel + signed checksum\nPyPI + djangoproject.com" },
  ],
  edges: [
    { from: "change", to: "ticket", label: "non-trivial issue filed; accepted ticket" },
    { from: "change", to: "pr", label: "push branch; open PR" },
    { from: "ticket", to: "pr", label: "ticket number / discussion linked" },
    { from: "pr", to: "review", label: "patch + tests/docs for review" },
    { from: "review", to: "change", label: "flags / comments; revise and push" },
    { from: "review", to: "pr", label: "review complete; merger checks PR" },
    { from: "review", to: "main", label: "Ready for checkin; merger commits" },
    { from: "main", to: "stable", label: "feature freeze forks stable branch" },
    { from: "stable", to: "release", label: "green Jenkins; release from stable branch" },
    { from: "release", to: "artifacts", label: "tag, build, sign, upload; activate entry" },
    { from: "artifacts", to: "release", label: "published version; announce release" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string }>;
