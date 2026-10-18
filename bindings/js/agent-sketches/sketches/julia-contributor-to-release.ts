// Julia — a contributor's change through review, backport validation, and an official point release.
// Sources: https://github.com/JuliaLang/julia/blob/master/CONTRIBUTING.md
//          https://github.com/JuliaLang/julia/blob/master/doc/src/devdocs/build/distributing.md
//          https://github.com/JuliaCI/julia-buildkite/blob/main/ops/README.md
// Follows a bug fix selected for a patch release from the contributor's branch through release-x.y,
// then traces tagged CI artifacts through signing, promotion, version discovery, and announcement.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  parent?: string;
  fill?: string;
  width?: number;
};

const sketch = {
  nodes: [
    { id: "contributor", class: "box", x: 20, y: 170, label: "Contributor\nfork Julia; build locally; keep current with master", fill: "#eff6ff" },
    { id: "change", class: "box", x: 250, y: 70, label: "Feature branch\none logical fix + tests" },
    { id: "pr", class: "box", x: 250, y: 260, label: "GitHub pull request\nJuliaLang/julia ← fork branch" },
    { id: "review", class: "box", x: 500, y: 70, label: "Community + committers\nreview, discuss, approve" },
    { id: "prci", class: "box", x: 500, y: 260, label: "julia-pr Buildkite\nuntrusted PR build; no signing secrets", fill: "#fff7ed" },
    { id: "revise", class: "box", x: 500, y: 430, label: "Contributor update\naddress review or failing tests", fill: "#eff6ff" },
    { id: "master", class: "box", x: 760, y: 170, label: "master\nmerged change" },
    { id: "backportlabel", class: "box", x: 980, y: 20, label: "PR label\nbackport x.y" },
    { id: "backporter", class: "box", x: 1200, y: 20, label: "Backporter\ncherry-pick merged commit" },
    { id: "backportbranch", class: "box", x: 1420, y: 20, label: "backports-release-x.y\nBackporter checklist + pending PRs" },
    { id: "manual", class: "box", x: 1200, y: 150, label: "Manual backport\ngit cherry-pick -x; resolve conflicts" },
    { id: "backportpr", class: "box", x: 1640, y: 20, label: "Backports PR\npush to backports-release-x.y" },
    { id: "nanosoldier", class: "box", x: 1640, y: 160, label: "Nanosoldier\nrunbenchmarks(ALL, vs=release-x.y)" },
    { id: "pkgeval", class: "box", x: 1640, y: 300, label: "Nanosoldier → PkgEval\nruntests(ALL, vs=release-x.y)" },
    { id: "investigate", class: "box", x: 1420, y: 390, label: "Investigate regressions\nfix/drop backport; rerun checks" },
    { id: "bkci", class: "box", x: 1880, y: 20, label: "julia-ci Buildkite\nunit tests + binaries, supported platforms" },
    { id: "gates", class: "box", x: 2120, y: 170, label: "Patch release gates\nunit tests pass; no perf or package regressions" },
    { id: "releasebranch", class: "box", x: 2360, y: 20, width: 300, label: "release-x.y\nmerge validated backports" },
    { id: "labelaudit", class: "box", x: 2120, y: 20, label: "Backport Label Audit\nGitHub Actions removes completed labels" },
    { id: "versionpr", class: "box", x: 2360, y: 190, label: "VERSION PR\nremove -pre on release-x.y" },
    { id: "tag", class: "box", x: 2660, y: 190, label: "Release manager\ngit tag v$(cat VERSION); push --tags" },
    { id: "posttagpr", class: "box", x: 2660, y: 360, label: "Next patch PR\nbump patch version; restore -pre" },
    { id: "ci", class: "box", x: 2900, y: 190, width: 340, label: "julia-ci Buildkite (tag vx.y.z)\nall-platform binaries + tests + light/full source tarballs + HTML docs", fill: "#fff7ed" },
    { id: "stage", class: "box", x: 3260, y: 20, label: "CI staging bucket\nversion artifacts; tag-specific build" },
    { id: "testgate", class: "box", x: 3260, y: 190, label: "Tag build tests\npass before publish trigger" },
    { id: "publish", class: "box", x: 3500, y: 190, width: 300, label: "Trusted julia-publish\nverify trusted commit; sign artifacts" , fill: "#fef3c7" },
    { id: "kms", class: "box", x: 3500, y: 380, label: "Cloud KMS / signing services\nGPG; macOS notarization; Windows Authenticode" },
    { id: "nightlies", class: "box", x: 3840, y: 190, label: "julialangnightlies bucket\nsigned version-named binaries + docs" },
    { id: "promote", class: "box", x: 4080, y: 190, width: 300, label: "Release manager starts julia-promote\nbranch vx.y.z; copy artifacts + purge CDN" },
    { id: "releasebucket", class: "box", x: 4420, y: 190, width: 300, label: "julialang-s3.julialang.org\nrelease layout; latest pointers; SHA256/MD5" },
    { id: "versions", class: "box", x: 4760, y: 20, label: "VersionsJSONUtil.jl CI\nregenerate versions.json" },
    { id: "juliaup", class: "box", x: 4760, y: 170, label: "juliaup Update Version DB\nupload jobs manually approved" },
    { id: "githubrelease", class: "box", x: 4760, y: 330, label: "contrib/github_source_release.sh\nverify signatures + layout; create GitHub release" },
    { id: "website", class: "box", x: 5080, y: 190, label: "Julia website\nconfig.md version; regenerate old releases" },
    { id: "announce", class: "box", x: 5320, y: 190, label: "Published Julia point release\nDiscourse announcement + downloadable binaries" },
  ] satisfies Node[],
  edges: [
    { from: "contributor", to: "change", label: "implement, build, test" },
    { from: "change", to: "pr", label: "push branch; open PR" },
    { from: "pr", to: "review", label: "code + test changes" },
    { from: "pr", to: "prci", label: "PR event; untrusted pipeline" },
    { from: "review", to: "revise", label: "requested changes" },
    { from: "prci", to: "revise", label: "failed build/test feedback" },
    { from: "revise", to: "pr", label: "push revision; re-review" },
    { from: "review", to: "master", label: "approved; merge by committer" },
    { from: "prci", to: "master", label: "checks pass" },
    { from: "master", to: "backportlabel", label: "select applicable fix for patch x.y" },
    { from: "backportlabel", to: "backporter", label: "backport x.y label" },
    { from: "backporter", to: "backportbranch", label: "cherry-pick commit; -x records source SHA" },
    { from: "backporter", to: "manual", label: "does not apply cleanly" },
    { from: "manual", to: "backportbranch", label: "resolved cherry-pick" },
    { from: "backportbranch", to: "backportpr", label: "branch update + checklist" },
    { from: "backportpr", to: "nanosoldier", label: "benchmark request vs release-x.y" },
    { from: "backportpr", to: "pkgeval", label: "test registered packages vs release-x.y" },
    { from: "nanosoldier", to: "investigate", label: "regression summary" },
    { from: "pkgeval", to: "investigate", label: "new package failures + logs" },
    { from: "investigate", to: "backportpr", label: "fix/drop commits; rerun tests" },
    { from: "backportpr", to: "bkci", label: "every push builds/tests supported platforms" },
    { from: "bkci", to: "gates", label: "unit test results + artifacts" },
    { from: "nanosoldier", to: "gates", label: "no real performance regression" },
    { from: "pkgeval", to: "gates", label: "no newly broken registered packages" },
    { from: "gates", to: "releasebranch", label: "all release checks pass; merge backports" },
    { from: "releasebranch", to: "labelaudit", label: "backports merged" },
    { from: "releasebranch", to: "versionpr", label: "last branch change: remove -pre" },
    { from: "versionpr", to: "tag", label: "merged VERSION update" },
    { from: "tag", to: "ci", label: "push vX.Y.Z tag triggers tag build" },
    { from: "tag", to: "posttagpr", label: "start next patch line" },
    { from: "ci", to: "stage", label: "commit-keyed, tag-specific artifacts" },
    { from: "ci", to: "testgate", label: "platform test results" },
    { from: "testgate", to: "publish", label: "only on passing tests; auto-trigger" },
    { from: "publish", to: "kms", label: "remote signing operations via OIDC-authorized job" },
    { from: "stage", to: "publish", label: "binaries + source distributions + docs" },
    { from: "kms", to: "publish", label: "signatures / notarization" },
    { from: "publish", to: "nightlies", label: "signed artifacts + deployed docs" },
    { from: "nightlies", to: "promote", label: "all per-platform publish builds complete" },
    { from: "promote", to: "releasebucket", label: "copy artifacts; checksums; latest pointers; CDN purge" },
    { from: "releasebucket", to: "versions", label: "release metadata inputs" },
    { from: "versions", to: "juliaup", label: "regenerated versions.json; dispatch DB update" },
    { from: "releasebucket", to: "githubrelease", label: "download signed source tarballs; verify; --execute" },
    { from: "githubrelease", to: "website", label: "published release version" },
    { from: "website", to: "announce", label: "website updated" },
    { from: "juliaup", to: "announce", label: "new version discoverable in juliaup" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
