# ノードの見た目を、SVG に近い DSL で描く関数として渡す

## きっかけ

`render()` に渡せるノードは矩形と円だけで、DFD のデータストア（ID の欄と区切り線があり、右が開いた形）のような形は描けない。形ごとに `shape` を足していくと、nicevg が DFD や C4 の記法を全部抱えることになる。

形を描くのは利用者（多くはエージェント）に任せ、nicevg は配置と経路だけを受け持ちたい。描く側はエージェントなので、新しい書き方を覚えさせず、SVG をそのまま書く感覚で書けるほうがよい。エージェントは SVG を書き慣れている。

## 案

### 使い方

ノードに `class` を付け、`style` に同じ名前の関数を渡す。関数はノードを受け取り、原点を基準に描いた SVG を返す。

```js
import { render, svg, text } from "nicevg";

const image = render({
  nodes: [
    { id: "customer", class: "external", x: 20,  y: 40,  label: "顧客" },
    { id: "order",    class: "process",  x: 220, y: 40,  label: "注文を受ける", no: "1" },
    { id: "orders",   class: "store",    x: 460, y: 120, label: "注文",         no: "D1" },
  ],
  edges: [
    { from: "customer", to: "order",  label: "注文内容" },
    { from: "order",    to: "orders", label: "注文" },
  ],
  style: {
    store: ({ no, label }) => {
      const id = text(no), name = text(label);
      const sep = id.width + 24, w = sep + name.width + 24, h = Math.max(id.height, name.height) + 20;
      return svg`
      <g fill="none" stroke="#333" stroke-width="1.5">
        <path d="M ${w} 0 H 0 V ${h} H ${w}"/>
        <line x1="${sep}" y1="0" x2="${sep}" y2="${h}"/>
      </g>
      ${id.at({ cx: sep / 2, cy: h / 2 })}
      ${name.at({ left: sep + 12, cy: h / 2 })}`;
    },
    // external、process も同じ形で書く
  },
});
```

### DSL

DSL はタグ付きテンプレート `` svg`...` `` と `text()` の 2 つだけにする。

- `` svg`...` `` の中は SVG そのもので、計算した座標は `${}` で埋め込む。埋め込んだ文字列はエスケープされる。`svg` で作った断片はエスケープされないので、入れ子にできる。配列も埋め込める。
- `text(文字列, 大きさ)` は `<text>` 要素そのものを表す。nicevg と同じ見積もり（全角はフォントサイズ、それ以外は 0.59 倍）で `width`、`height`、`size` を持ち、`<text>` を手で書かなくてよい。
- 文字は `.at({ 横, 縦 })` で置く。横は `left` / `cx` / `right`、縦は `top` / `cy` / `bottom` から 1 つずつ選び、文字の箱のその点をその座標に合わせる。`cx`・`cy` は SVG の `<circle cx cy>` と同じ名前。同じ軸を 2 つ渡したときや、片方の軸がないときはエラーにする。
  - 出力は、横を `text-anchor`（`start` / `middle` / `end`）に、縦は常に `dominant-baseline="central"` にし、`top`・`bottom` は高さから中心を求める。`hanging` などは描画ソフトで効き方が違うため使わない。
  - `<g transform>` で包まず、`<text>` の `x`・`y` に直接書く。
- 関数は原点を基準に描き、置く位置は書かない。`render` がノードの `x`、`y` と、arrange の移動を `<g data-node="…" transform="translate(…)">` で付ける。`svg` で作った断片も `.at(x, y)` で置ける（左上を (x, y) へ）。

採らなかった書き方:

- 組み立て用の部品（`frame(row(pad(...)))` のような入れ子）や、属性をオブジェクトで渡す書き方（`path({ d })`）: SVG から離れる。
- 位置を決めるメソッドの連鎖（`name.left(x).cy(y)`）: 括弧が少なく短いが、片方の軸を書き忘れても 0 として描かれ、気づきにくい。
- `.mid(x, y)`・`.centerAt(x, y)`・`.leftAt(x, y)`: 基準点の種類ごとにメソッドが増え、横と縦を別々に選べない。
- `center` と `middle` で横と縦の中心を分ける: どちらがどちらか分からない。
- `setLeft()`・`setCX()`: 元の値を書き換えるように読めるが、実際は新しい値を返す。

### 決めたこと

1. **ノードの範囲は、描いた結果から nicevg が求める。** 関数は大きさを返さない。DSL を書くのが楽なほうを採る。範囲は、関数が返した要素（`rect`、`circle`、`line`、`polygon`、`M`・`L`・`H`・`V` の `path`、見積もった `text`）の外接矩形とする。今の SVG の読み取りにある部品を使えるので、Rust の側で求める。
2. **カスタムノードは大きさを変えない。** 関数に `width` と `height` を渡して描き直させる形にすると、関数が「渡された大きさに合わせて描く」ことを考えなければならず、書くのが重くなる。そのため、修正も配置調整もカスタムノードを動かすだけにする。
   - ラベルの余白の検査と、ボックスの拡張は行わない。中の `<text>` は関数が描いた模様の一部として扱い、ノードのラベルとは見なさない。
   - ノードの押し出し（重なり・間隔の不足）と arrange の移動は、今までどおり `translate` で行う。
   - コネクタは外接矩形の辺から出入りする。データストアの開いた右辺にも、右辺があるものとしてつなぐ。

### 未決: `edges` と `connectors`

グラフを扱うライブラリの入力は `nodes` と `edges` が多い（Cytoscape.js、React Flow、dagre、ELK、vis-network）。D3 の力学レイアウトと Mermaid は `links` を使う。グラフ理論の用語としても `edges` が通じやすい。

一方で nicevg は、README・SVG の約束・Rust の型・返す図（`Diagram.connectors`）のどこでも「コネクタ」と呼んでいる。

案:

- `render()` の入力は `edges` を正式な名前にし、`connectors` も同じ意味で受け付ける（0.15 までの呼び出しを壊さない）。両方渡されたらエラーにする。
- 返す図や型の名前（`SketchConnector`、`Diagram.connectors`）を `edges` に揃えるかは、入力の名前を変えたあとで決める。中だけ揃えるとかえって紛れるので、変えるなら一度に変える。

## 気をつけること

- 外接矩形の外に線がはみ出す形（太い線、矢印の飾りなど）は、範囲からはみ出したぶんがノードの押し出しや経路の障害物に入らない。線の太さの半分を範囲に足すかは、実際の図で確かめる。
- `A`（円弧）や曲線を含む `path` は今の読み取りで範囲を求められない。これらを範囲の計算から外すか、エラーにするかを決める。
- 関数は JavaScript の中で呼び、描いた SVG を文字列としてネイティブ側に渡す。Rust の `Sketch` に「中身の SVG を持つノード」を足すことになる。Rust から使う場合は、SVG の文字列を直接書く。
- 描いた中身が大きいと、ノードの中で文字が切れていても nicevg は気づかない。大きさを変えないと決めたので、これは関数を書く側の責任になる。

## 結果（2026-10-14、実装）

- TypeScript: `svg`・`text`・`SvgFragment`・`Text` を `bindings/js/src/draw.ts` に置き、`render()` が `class` を持つノードを `style` の関数で描く。`edges` を正式な名前にし、`connectors` も受け付け、両方渡すと `NicevgError` にした。
- Rust: `SketchNode` に `svg` を足し、`width`・`height` を省けるようにした（`svg` がなければ `MissingSize` のエラー）。描いた SVG は `<g data-node="…" data-custom="true"><g transform="translate(x y)">…</g></g>` に入れる。`translate` を絵だけに付けるので、子ノードは渡した位置のまま親の中に入る。整形式でない SVG や何も描かない SVG は `InvalidSvg` のエラー。
- SVG の約束に `data-custom="true"` を足した。範囲は中の要素（入れ子のノードを除く）の外接矩形（`diagram::content_bounds`）で、中の `<text>` はラベルにせず、中のグループもノードと推定しない。ラベルがないので、修正の拡大も起きない。
- 4 つの例の図の修正のスナップショットは変わらなかった。
- 線の太さの半分を範囲に足すことと、曲線の膨らみを範囲に入れることは、まだしていない（曲線と円弧は端点だけで数える）。

## 残っていること

- `StyledNode` の項目は `unknown` なので、TypeScript ではスタイル関数の中で `text(String(label))` のように変換が要る。`style` のキーとノードの `class` を型引数にし、mapped type でクラスごとのノードの項目を結び付ければ、`label` や `no` の型がクラスから決まり、変換も要らなくなる。
