# nicevg for TypeScript (Bun and Node)

nicevg で図を SVG に描く。TypeScript や JavaScript から、Bun では `bun:ffi`、Node では [koffi](https://koffi.dev/) でネイティブライブラリを呼ぶ。図はノードとコネクタの配列（Sketch）のほか、SVG の文字列でも渡せる。処理は Rust のネイティブコードで動くので、CLI と同じ速さで、出力も同じになる。

## インストール

```sh
npm install nicevg   # または bun add nicevg
```

パッケージには、次の環境向けにビルド済みのネイティブライブラリが入っているので、インストール時にスクリプトは走らず、Rust も要らない。

| 環境 | 対象 |
|---|---|
| Linux | x64、arm64（glibc 2.17 以降） |
| macOS | arm64、x64 |
| Windows | x64 |

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
| `nodes[].class` | 描き方を `style` から選ぶ名前。付けたノードは `width`・`height`・`shape`・`label` を使わず、`style` が描く |
| `edges[].from`、`to` | 両端のノードの id。`edges` は前の名前の `connectors` でも渡せる（両方は渡せない） |
| `edges[].points` | 経路の折れ点。省くと、修正と配置調整が両端のノードの辺から辺へ経路を引く |
| `edges[].label`、`fontSize` | コネクタの脇に置く文字 |
| `style` | クラス名から、そのクラスのノードを描く関数への対応 |
| `rootAttrs` | ルートの `<svg>` に付ける属性（`{ "font-family": "'Yu Gothic', sans-serif" }` など）。中の文字はこれを継承する。`xmlns`・`viewBox`・`width`・`height` は nicevg が書くので渡せない |

### style

`style` の関数はノードを受け取り、ノードの左上を原点にして描いた SVG を `svg` で返す。ノードに足した項目（下の `no` など）も受け取れる。置く位置は `render` がノードの `x`、`y` と配置調整の移動から `translate` で付ける。

```ts
import { render, svg, text } from "nicevg";

const image = render({
  nodes: [
    { id: "customer", x: 20, y: 40, width: 80, height: 40, label: "顧客" },
    { id: "orders", class: "store", x: 260, y: 40, no: "D1", label: "注文" },
  ],
  edges: [{ from: "customer", to: "orders", label: "注文" }],
  style: {
    store: ({ no, label }) => {
      const id = text(String(no)), name = text(String(label));
      const sep = id.width + 24, w = sep + name.width + 24, h = Math.max(id.height, name.height) + 20;
      return svg`
        <g fill="none" stroke="#333">
          <path d="M ${w} 0 H 0 V ${h} H ${w}"/>
          <line x1="${sep}" y1="0" x2="${sep}" y2="${h}"/>
        </g>
        ${id.at({ cx: sep / 2, cy: h / 2 })}
        ${name.at({ left: sep + 12, cy: h / 2 })}`;
    },
  },
});
```

- `` svg`...` `` の中は SVG そのもの。埋め込んだ文字列はエスケープされ、`svg` の断片、`text`、それらの配列はそのまま入る。断片は `.at(x, y)` で原点をずらせる。
- `text(文字列, 大きさ)` は 1 行の `<text>`。`width`・`height` は nicevg と同じ見積もり。`.at({ 横, 縦 })` で、横は `left`・`cx`・`right`、縦は `top`・`cy`・`bottom` から 1 つずつ選び、文字の箱のその点をその座標に置く。
- ノードの範囲は、描いたもの（`rect`、`circle`、`ellipse`、`line`、`polyline`、`polygon`、`path`、`text`）を囲む矩形。コネクタはその辺から出入りする。曲線と円弧は端点だけで数える。
- 修正と配置調整はノードを動かすが、大きさは変えない。中の `<text>` はラベルとして検査しないので、収まりは描く側が決める。

`Svg` の `report.diagram` は、描いた図を読み直したもので、ノード（`bounds`、ラベルの範囲 `labelBounds`）、コネクタ（`points`）、ラベル（`bounds`、説明するコネクタの id `connector`）を持つ。コネクタは id を持たないので、`from` と `to` で見分ける。文字の幅は、全角を 1 文字あたりフォントサイズ、それ以外を 0.59 倍として見積もる。

形の書き方が違う入力、存在しないノードを指す `from`・`to`・`parent`、`style` にないクラス、整形式でない SVG を描いたスタイルは、どこが何を指しているか（`connectors[0].to`、`nodes[1].class` など）を書いた `NicevgError` を投げる。

描画は呼んだスレッドで最後まで走る。大きな図の配置調整は 1〜2 秒かかり、その間すべてのコアを使う。

## 開発

Bun と Node（型を取り除いて .ts を動かせる 22.18 以降）、Rust のツールチェーンが要る。

```sh
bun install
bun run build       # cargo でネイティブライブラリを作って native/ に置き、tsc で src/ を dist/ に書き出す
bun run test        # Bun で src/ を、Node でビルドした dist/ をテストする
bun run typecheck
```

パッケージは Bun には `src/` の TypeScript を、Node には `dist/` の JavaScript と型定義を渡す（`package.json` の `exports`）。ネイティブライブラリは `NICEVG_LIBRARY` 環境変数、`native/<platform>-<arch>/`（`bun run build` が置く場所で、公開するパッケージでは全環境ぶん）、リポジトリの `target/release/` の順に探す。

`v*` のタグを push すると、`.github/workflows/npm.yml` が各環境のネイティブライブラリを作ってパッケージに入れ、Bun と Node でテストしてから npm に公開する。タグの版と `package.json` の `version` が違えば止まるので、リリースのときは Rust の `Cargo.toml`（ルートと `bindings/js`）と一緒に `package.json` の版も上げる。手で実行すると、公開せずにビルドとパックだけを確かめられる。

公開は npm の Trusted Publishing（GitHub Actions の OIDC）で、npm のトークンを使わず、配布元の証明（provenance）も自動で付く。

Trusted Publisher はすでに npm にあるパッケージにしか登録できないので、最初の 1 回だけは手元から公開する。npm のアカウントで 2FA を有効にしたうえで、次のようにする。

1. Actions の「Publish to npm」を手で実行する（手での実行は公開しない）。全環境のライブラリをそれぞれの環境でテストし、まとめたパッケージが、成果物 `npm-package` に残る。
2. それを手元に落として公開する（2FA のワンタイムコードを聞かれる）。手元からの公開なので、この版には配布元の証明が付かない。
   ```sh
   gh run download <run-id> -n npm-package
   npm publish ./nicevg-<version>.tgz
   ```
3. このリポジトリと `npm.yml` を Trusted Publisher に登録する（npmjs.com のパッケージの設定か、次のコマンド。どちらも 2FA が要る）。
   ```sh
   npm trust github nicevg --file npm.yml --repo takagiy/nicevg --allow-publish
   ```

以後はタグの push だけで公開される。npm のトークンは作らない（2FA を回避するトークンは、npm が段階的に廃止している）。
