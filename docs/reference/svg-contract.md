# SVG の約束

nicevg が SVG のどこを図として読むか。ここに書いた形の SVG なら、検査と修正の対象になる。

## ノードとコネクタ

通常の `<g>` に直接 `<rect>` または `<circle>` と `<text>` が入っていればノードとして推定する。LLM から安定した SVG を受け取る場合は、次の属性で意味を明示する。

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

- ノードの形は `<rect>` または `<circle>`。円形ノードは外接矩形を `bounds` とし、`shape: "circle"` で区別する。ラベルの余白、コネクタの横切りと端点は円そのもので判定し、修正では中心を保ったまま半径を広げ、コネクタの端を円周まで伸ばす。
- 構造図の包含は、子の `data-node` グループを親の `data-node` グループ内にネストする。
- 意図したノード重複には `data-allow-overlap="true"` を付ける。
- `data-node` グループに `data-custom="true"` を付けると、中身を絵として扱う。修正と配置調整は `translate` で動かすだけで、大きさは変えない。中のグループはノードと推定しない（入れ子の `data-node` は子ノード）。
  - 範囲と形: 絵の中に `data-bounds="true"` を付けた `<rect>` か `<circle>` があれば、それがノードの範囲と形になる（`<circle>` なら円形のノード）。なければ、中の要素（`rect`、`circle`、`ellipse`、`line`、`polyline`、`polygon`、`path`、`text`）を囲む矩形（入れ子のノードは除く）を範囲にする。
  - 中の `<text>` は、すべてそのノードのラベルの範囲として読み、配置調整はほかのノードをそれに近づけない。ラベルが内側の余白に収まるかは調べない（大きさは絵を描いた側が決める）。
- 自動経路修正の対象となるコネクタには `data-from` と `data-to` を付ける。向きのない接続には同じ意味で `data-a` と `data-b` も使える。
- コネクタのパスは `M`、`L`、`H`、`V` による折れ線を使用する。
- 矢じりを `<polygon>` で描く場合は、線と矢じりを `data-from` と `data-to` を付けた `<g>` で囲むか、注釈付きの線と同じ `<g>` に矢じりを置く。線の端に接する多角形を矢じりとみなし、コネクタは矢じりの先端まで続くものとして扱う。引き直すと、矢じりは新しい終点へ移って最後の区間の向きに回り、線は元と同じだけ手前で止まる。

## ラベル

- ノード外のラベルには `data-label="label-id"` を付ける。
- コネクタの説明ラベルには `data-label-for="connector-id"` も付ける。コネクタを引き直すと、ラベルはその脇へ移動する。置き場所の選び方は [テストリスト](../test-list/diagram.md) の「決定論的な修正」にある。
- 複数行のラベルは、`x` を持つ `<tspan>` で行を分けた 1 つの `<text>`、同じ `data-label` を持つ複数の `<text>`、`data-label` を付けた `<g>` で囲んだ複数の `<text>` のいずれかで書ける。移動するときはすべての行が一緒に動く。

## 座標とテキストの見積もり

- 座標変換は `translate(...)` だけを読む。
- テキストの幅は、全角文字を 1 文字あたりフォントサイズ、それ以外を 0.59 倍として見積もる。`font-size`、`text-anchor`、`dominant-baseline` は属性と `style` 属性の両方から読み、祖先のグループから継承する。`<style>` 要素のスタイルシートは読まない。

## 対応しない形

対応できない自由形状には意味を推測せず、`unsupportedElements` としてレポートする。自動修正でもその要素は変更しない。
