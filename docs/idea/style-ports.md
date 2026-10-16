# エッジの出入りする位置（ポート）を style で決める

## きっかけ

今はコネクタの出入りする辺も、辺の上の位置も nicevg が選ぶ。次のような図では、決まった位置から出入りしないと意味が通らない。

- フローチャートのひし形: 上の頂点から入り、右と下の頂点から「はい」「いいえ」で出る。
- ER 図: 表の中の列から、別の表の列へリレーションの矢印を引く。表の左辺か右辺の、その列の行の高さから出入りする。

エッジは次のように書きたい。

```ts
{ from: "article.author", to: "user.userId" }
```

[基本図形も style で描く](basic-shapes-as-styles.md) の `data-bounds`（ノードの範囲を示す）の上に乗る話なので、その後に進める。

## 案

### style がポートを宣言する

style の絵の中に、名前を付けた点としてポートを置く。

```svg
<!-- ひし形: 頂点に置く。向きは範囲の辺から決まる（上の頂点なら上向き） -->
<g data-node="check" data-custom="true">
  <g transform="translate(200 100)">
    <rect data-bounds="true" width="120" height="80" fill="none" stroke="none"/>
    <polygon points="60 0 120 40 60 80 0 40" fill="white" stroke="black"/>
    <circle data-port="in" cx="60" cy="0" r="0"/>
    <circle data-port="yes" cx="120" cy="40" r="0"/>
    <circle data-port="no" cx="60" cy="80" r="0"/>
  </g>
</g>

<!-- ER 図: 列の行の高さに、左右 2 点を同じ名前で置く -->
<g data-node="article" data-custom="true">
  <g transform="translate(20 20)">
    <rect data-bounds="true" width="160" height="96" fill="white" stroke="black"/>
    …
    <circle data-port="author" cx="0" cy="62" r="0"/>
    <circle data-port="author" cx="160" cy="62" r="0"/>
  </g>
</g>
```

- ポートは範囲の辺の上の点とする。出る向きは、その点が乗っている辺の外向きなので、指定は要らない。
- 同じ名前の点が複数あれば、経路探索がそのうち良いほうを選ぶ。ER 図の列は左右どちらからでもつなげる。
- 点は半径 0 の `<circle>` で書く。DSL に `port("author", { left: 0, cy: 62 })` のような補助関数を足せる。

### エッジの書き方

- TypeScript: `{ from: "article.author", to: "user.userId" }`。
- SVG の約束: `data-from` とは別の属性にする（`data-from="article" data-from-port="author"`、`data-to-port` も同じ）。ノードの id に `.` が入っていると `"a.b.c"` をどこで分けるか決められないため。
- TypeScript の文字列表記は最後の `.` で分ける（ノードの id に `.` を使える）。全体がノードの id に一致するときは、そのノードにポートなしでつなぐ。`{ from: { node: "article", port: "author" } }` の形は、要るまで受け付けない。

### 経路探索

- ポートを指定した端は、辺の選択（`choose_sides`）と端点の並べ直し（`spread_side`）の対象から外し、その点と向きで固定する。辺の候補は、そのポートの点だけになる。
- 同じ辺のほかの端点は、固定されたポートを避けて並べる。`spread_ports` には、引き直さないコネクタの端点を `fixed` として渡す仕組みがすでにあるので、それを使う。
- 向かい合う辺をつなぐときの直線化は、固定した端の位置に合わせて働く。

### 検査と配置調整

- 端点が宣言したポートにないときの問題を足し、修正がそれを引き直す。
- 端点の広がりの評価（`port_unevenness`・`lone_port_offset`）からは、ポートの端を外す。ER 図の列の位置は図の意味なので、偏っていても悪くない。

### 同じポートに来る複数のエッジ

ポートで合流させる。どれも同じ点に入り、ポートの手前で重なるのは問題にしない。ER 図の列は 1 行なので、意味とも合う。

### 結果

`report` のコネクタに `fromPort`・`toPort` を足す。

## 決めていないこと

- ひし形の範囲は外接矩形のままなので、ほかの線がひし形の外側の三角形の空き地を通れない（安全な側にずれるだけ）。`<polygon data-bounds>` で形そのものを範囲にするのは、さらに先の話にする。
- ポートのないノードとエッジは、今どおり自由に辺を選ぶ。
