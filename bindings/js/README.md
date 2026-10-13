# nicevg for TypeScript (Bun and Node)

nicevg で図を SVG に描く。TypeScript や JavaScript から、Bun では `bun:ffi`、Node では [koffi](https://koffi.dev/) でネイティブライブラリを呼ぶ。図はノードとコネクタの配列（Sketch）のほか、SVG の文字列でも渡せる。処理は Rust のネイティブコードで動くので、CLI と同じ速さで、出力も同じになる。

## 使い方

入口は `render()` 1 つ。図を受け取り、配置の問題を直し、ノードを少し動かして折れ・交差・混雑を減らして、SVG を表す `Svg` を返す。

```ts
import { render } from "nicevg";

const svg = render({
  nodes: [
    { id: "order", x: 20, y: 20, width: 120, height: 56, label: "Order" },
    { id: "bill", x: 300, y: 160, width: 120, height: 56, label: "Bill" },
    { id: "user", x: 40, y: 200, width: 80, height: 80, shape: "circle", label: "User" },
  ],
  connectors: [
    { from: "order", to: "bill", label: "invoice" },
    { from: "user", to: "order" },
  ],
});

await svg.write("diagram.svg"); // ファイルに書く
const text = svg.toString(); // SVG の文字列（`${svg}` でも同じ）

for (const node of svg.report.diagram.nodes) {
  console.log(node.id, node.bounds); // 動いた後の位置と大きさ
}
for (const connector of svg.report.diagram.connectors) {
  console.log(connector.from, connector.to, connector.points); // 両端のノードと、直交する経路の折れ点
}
console.log(svg.report.issues); // 残った問題（なければ空）
console.log(svg.changes); // 何を直し、何を動かしたか
```

- `input` は `Sketch`、`{ svg: string }`、または前に返った `Svg`。

### Sketch

| 項目 | 内容 |
|---|---|
| `nodes[].id`、`x`、`y`、`width`、`height` | ノードの id と、左上の位置と大きさ |
| `nodes[].shape` | `"rect"`（既定）か `"circle"`。円は外接する正方形を `x`〜`height` で渡す |
| `nodes[].label`、`fontSize` | 中に書く文字（`\n` で改行）。子を持つノードでは左上に、それ以外では中央に置く |
| `nodes[].parent` | このノードを入れる親ノードの id |
| `nodes[].allowOverlap` | 意図した重なりなら `true` |
| `connectors[].from`、`to` | 両端のノードの id |
| `connectors[].points` | 経路の折れ点。省くと、修正と配置調整が両端のノードの辺から辺へ経路を引く |
| `connectors[].label`、`fontSize` | コネクタの脇に置く文字 |

`Svg` の `report.diagram` は、描いた図を読み直したもので、ノード（`bounds`、ラベルの範囲 `labelBounds`）、コネクタ（`points`）、ラベル（`bounds`、説明するコネクタの id `connector`）を持つ。コネクタは id を持たないので、`from` と `to` で見分ける。文字の幅は、全角を 1 文字あたりフォントサイズ、それ以外を 0.59 倍として見積もる。

形の書き方が違う入力や、存在しないノードを指す `from`・`to`・`parent` は、どこが何を指しているか（`connectors[0].to` など）を書いた `NicevgError` を投げる。

描画は呼んだスレッドで最後まで走る。大きな図の配置調整は 1〜2 秒かかり、その間すべてのコアを使う。

## 開発

Bun と Node（型を取り除いて .ts を動かせる 22.18 以降）、Rust のツールチェーンが要る。

```sh
bun install
bun run build       # cargo でネイティブライブラリを作って native/ に置き、tsc で src/ を dist/ に書き出す
bun run test        # Bun で src/ を、Node でビルドした dist/ をテストする
bun run typecheck
```

パッケージは Bun には `src/` の TypeScript を、Node には `dist/` の JavaScript と型定義を渡す（`package.json` の `exports`）。ネイティブライブラリは `NICEVG_LIBRARY` 環境変数、`native/`、リポジトリの `target/release/` の順に探す。
