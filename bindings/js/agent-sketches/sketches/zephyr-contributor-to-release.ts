// Zephyr Project. Sources: https://docs.zephyrproject.org/latest/develop/contribute/index.html
// and https://docs.zephyrproject.org/latest/project/release_process.html
// A contributor's signed-off change passes PR checks and review during the merge window,
// then the release team stabilizes it, updates VERSION, signs a tag, and publishes the release.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "record" | "gate" | "artifact";
type N = SketchNode & { class: Kind; label: string; fill?: string };
const record = ({ label, fill = "#fffdf5" }: N) => {
  const lines = label.split("\n").map((line, i) => text(line, i === 0 ? 13 : 11));
  const w = Math.max(...lines.map((line) => line.width)) + 22;
  const h = lines.reduce((sum, line) => sum + line.height, 12);
  let y = 7;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="2" fill="${fill}" stroke="#475569"/>${lines.map((line, i) => {
    const el = line.at({ left: 10, top: y }); y += line.height;
    return i === 0 ? svg`<rect width="${w}" height="${line.height + 10}" fill="#dbeafe"/>${el}` : el;
  })}`;
};
const gate = ({ label }: N) => {
  const t = text(label, 12), w = t.width + 32, h = t.height + 30;
  return svg`<path data-bounds="true" d="M ${w/2} 0 L ${w} ${h/2} L ${w/2} ${h} L 0 ${h/2} Z" fill="#fef3c7" stroke="#92400e"/>${t.at({ cx: w/2, cy: h/2 })}`;
};
const artifact = ({ label }: N) => {
  const t = text(label, 12), w = t.width + 28, h = t.height + 18;
  return svg`<path data-bounds="true" d="M 0 0 H ${w-9} L ${w} 9 V ${h} H 0 Z" fill="#dcfce7" stroke="#166534"/>${t.at({ cx: w/2, cy: h/2 })}`;
};

const nodes: N[] = [
  { id:"patch",class:"record",x:30,y:60,label:"変更差分\n対象ファイル・目的" },
  { id:"commit",class:"artifact",x:195,y:40,label:"Git commit\n自己完結した変更" },
  { id:"dco",class:"record",x:180,y:145,label:"Signed-off-by\n作者名・メール" },
  { id:"branch",class:"artifact",x:365,y:65,label:"作業ブランチ\nupstream/main 起点" },
  { id:"pr",class:"record",x:520,y:50,label:"GitHub Pull Request\n変更説明・対象領域" },
  { id:"issue",class:"record",x:500,y:165,label:"Issue（任意）\n問題・影響・解決案" },
  { id:"ci",class:"gate",x:680,y:50,label:"PR 必須チェック" },
  { id:"gitlint",class:"record",x:680,y:155,label:"Gitlint\ncommit 形式" },
  { id:"identity",class:"record",x:835,y:5,label:"Identity / Emails\n作者情報" },
  { id:"license",class:"record",x:835,y:75,label:"License checks\nライセンス" },
  { id:"checkpatch",class:"record",x:835,y:145,label:"Checkpatch\nコーディング規約" },
  { id:"kconfig",class:"record",x:835,y:215,label:"Kconfig check\n設定記述" },
  { id:"devicetree",class:"record",x:980,y:5,label:"Device Tree check\nDTS・bindings" },
  { id:"docs",class:"record",x:980,y:75,label:"Documentation build\nSphinx" },
  { id:"lint",class:"record",x:980,y:145,label:"Code linters\nPylint・Ruff" },
  { id:"twister",class:"record",x:980,y:215,label:"Integration tests\ntwister・QEMU" },
  { id:"bt",class:"record",x:1120,y:145,label:"Simulated Bluetooth\ntests" },
  { id:"review",class:"gate",x:1120,y:55,label:"レビュー承認" },
  { id:"assignee",class:"record",x:1275,y:0,label:"指定 assignee\n承認の一つ" },
  { id:"maintainer",class:"record",x:1275,y:85,label:"該当領域 maintainer\nまたは collaborator" },
  { id:"orgreview",class:"record",x:1275,y:180,label:"異なる組織の承認\n共有コードの場合" },
  { id:"wait",class:"record",x:1430,y:60,label:"レビュー期間\n通常 2 営業日" },
  { id:"merge",class:"artifact",x:1585,y:65,label:"main merge commit" },
  { id:"window",class:"gate",x:1740,y:55,label:"merge window\nfeature 開発可" },
  { id:"rc1",class:"artifact",x:1900,y:40,label:"vX.Y.0-rc1\nfeature freeze" },
  { id:"freeze",class:"gate",x:2055,y:55,label:"安定化変更のみ\n新機能は原則不可" },
  { id:"blockers",class:"record",x:2055,y:175,label:"blocker issue\n全件解決が必要" },
  { id:"quality",class:"record",x:2210,y:0,label:"リリース品質基準\nTier 0/1 テスト" },
  { id:"samples",class:"record",x:2210,y:80,label:"samples / tests\nTier 0–2 build" },
  { id:"security",class:"record",x:2210,y:160,label:"静的解析・security\n重大指摘を解消" },
  { id:"notes",class:"artifact",x:2370,y:95,label:"Release Notes\n承認済み・merge" },
  { id:"version",class:"record",x:2520,y:20,label:"VERSION\nMAJOR・MINOR・PATCHLEVEL\nEXTRAVERSION = 空" },
  { id:"versionpr",class:"record",x:2705,y:55,label:"release: Zephyr X.Y.Z\nVERSION 更新 PR" },
  { id:"finalci",class:"gate",x:2875,y:55,label:"CI 成功後 merge" },
  { id:"tag",class:"artifact",x:3030,y:30,label:"署名付き annotated tag\nvX.Y.Z" },
  { id:"verify",class:"record",x:3200,y:0,label:"git show\nPGP / SSH 署名確認" },
  { id:"push",class:"artifact",x:3200,y:110,label:"git push\nzephyrproject-rtos/zephyr" },
  { id:"publish",class:"record",x:3370,y:45,label:"GitHub Releases\nrelease notes・docs link" },
  { id:"announce",class:"artifact",x:3540,y:60,label:"announce / devel\n公開告知メール" },
];
const edges = [
  ["patch","commit"],["commit","dco"],["dco","branch"],["branch","pr"],["issue","pr"],
  ["pr","ci"],["ci","gitlint"],["ci","identity"],["ci","license"],["ci","checkpatch"],["ci","kconfig"],["ci","devicetree"],["ci","docs"],["ci","lint"],["ci","twister"],["ci","bt"],
  ["ci","review"],["review","assignee"],["review","maintainer"],["review","orgreview"],["review","wait"],["wait","merge"],["merge","window"],["window","rc1"],["rc1","freeze"],
  ["freeze","blockers"],["freeze","quality"],["freeze","samples"],["freeze","security"],["blockers","notes"],["quality","notes"],["samples","notes"],["security","notes"],
  ["notes","version"],["version","versionpr"],["versionpr","finalci"],["finalci","tag"],["tag","verify"],["verify","push"],["push","publish"],["publish","announce"],
];
export default { nodes, edges: edges.map(([from,to]) => ({ from: from!, to: to! })), style: { record, gate, artifact }, rootAttrs: { "font-family":"sans-serif" } } satisfies Sketch<N>;
