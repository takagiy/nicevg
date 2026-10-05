# nicevg

SVG のフローチャート、構造図、仕組みの説明図に対して、座標に関する機械的な検査と決定論的な修正を行う TypeScript ツール。

LLM に座標の再計算や衝突確認をさせる代わりに、次の問題をプログラムで検出する。

- 描画範囲と安全余白が `viewBox` からはみ出している
- ラベルがノード内の 12px の余白に収まっていない
- 同じ段のノードが重なっている、または間隔が 20px 未満
- 自由ラベル同士が重なっている
- コネクタが無関係なノードを貫通している
- コネクタが自由ラベルから 8px 未満を通っている
- コネクタの端点が接続ノードの内側で止まっている

修正可能な問題には、`viewBox` の拡張、背景矩形の拡張、ノードの押し出し、コネクタの直交経路への引き直しを適用する。

## Setup

```sh
bun install
```

## CLI

```sh
# 人間向けの診断
bun run nicevg check diagram.svg

# 構造化 JSON
bun run nicevg check --json diagram.svg

# 標準入力を検査する（ファイル省略または -）
cat diagram.svg | bun run nicevg check
cat diagram.svg | bun run nicevg check -

# 修正後 SVG を標準出力へ出す
bun run nicevg fix diagram.svg

# 標準入力を修正して標準出力へ出す
cat diagram.svg | bun run nicevg fix > fixed.svg

# 別ファイルへ書く
bun run nicevg fix --output fixed.svg diagram.svg

# 入力ファイルを明示的に置き換える
bun run nicevg fix --write diagram.svg
```

`check` の終了コードは、問題なしが `0`、図の問題ありが `1`、不正な入力が `2`。
標準入力には置き換えるファイルパスがないため、`fix --write` とは併用できない。

## TypeScript API

```ts
import { analyze, fix } from "nicevg";

const report = analyze(svg);
const result = fix(svg);

console.log(report.issues);
console.log(result.svg, result.changes, result.report);
```

## SVG contract

通常の `<g>` に直接 `<rect>` と `<text>` が入っていればノードとして推定する。LLM から安定した SVG を受け取る場合は、次の属性で意味を明示する。

```xml
<g data-node="source">
  <rect x="20" y="20" width="120" height="56" />
  <text x="80" y="53" text-anchor="middle">Source</text>
</g>

<g data-node="target">
  <rect x="220" y="20" width="120" height="56" />
  <text x="280" y="53" text-anchor="middle">Target</text>
</g>

<path
  id="source-target"
  data-from="source"
  data-to="target"
  d="M 140 48 L 220 48"
/>
```

- 構造図の包含は、子の `data-node` グループを親の `data-node` グループ内にネストする。
- 意図したノード重複には `data-allow-overlap="true"` を付ける。
- ノード外のラベルには `data-label="label-id"` を付ける。
- 自動経路修正の対象となるコネクタには `data-from` と `data-to` を付ける。
- コネクタのパスは `M`、`L`、`H`、`V` による折れ線を使用する。
- 現在の座標変換は `translate(...)` をサポートする。

対応できない自由形状には意味を推測せず、`unsupportedElements` としてレポートする。自動修正でもその要素は変更しない。

## Development

自然言語のテストリストは `docs/test-list/diagram.md` に置く。実装は Canon TDD の縦方向のサイクルで進める。

```sh
bun test
bun run typecheck
bun run lint
bun run check
```
