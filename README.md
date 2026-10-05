# nicevg

SVG のフローチャート、構造図、仕組みの説明図に対して、座標に関する機械的な検査と決定論的な修正を行う Rust 製のツール。

LLM に座標の再計算や衝突確認をさせる代わりに、次の問題をプログラムで検出する。

- 描画範囲と安全余白が `viewBox` からはみ出している
- ラベルがノード内の 12px の余白に収まっていない
- 同じ段のノードが重なっている、または間隔が 20px 未満
- 自由ラベル同士が重なっている
- コネクタが無関係なノードを貫通している
- コネクタが自由ラベルから 8px 未満を通っている
- コネクタの端点が接続ノードの内側で止まっている
- コネクタ同士が同じ線分上で重なっている
- `data-label-for` で紐づいたラベルがコネクタから 16px より離れている

修正可能な問題には、`viewBox` の拡張、背景矩形の拡張、ノードの押し出し、コネクタの直交経路への引き直し、コネクタに紐づいたラベルの移動を適用する。

コネクタの引き直しでは、次を守る。

- 接続元と接続先の辺から垂直に出入りし、辺に沿って走らない
- 同じ辺の端点は辺上に分散させ、向かい合う辺をつなぐときは可能なら端点を揃えて直線にする
- 障害物から 8px 離れ、ほかのコネクタと同じ線分上に重ならない（必要なら 10px 隣に並走する）
- 1 回の迂回で避けられないときは、複数回折れる経路を探す
- 折れ曲がりが減るなら接続する辺を付け替える。ただし接続の多い辺は避ける

1 回の修正で問題が残るときは、問題が減る限り修正を繰り返す（既定で最大 5 回、`fix_with_passes(svg, max_passes)` で変更できる）。

## Install

```sh
brew install takagiy/tap/nicevg
# または
cargo install --git https://github.com/takagiy/nicevg
```

## CLI

標準入力の SVG を修正し、修正後 SVG を標準出力へ出す。

```sh
nicevg < diagram.svg > fixed.svg
```

修正しても残った問題は標準エラーへ表示する。終了コードは、問題が残らなければ `0`、問題が残れば `1`、不正な入力なら `2`。検査だけを行うときや構造化レポートが必要なときはライブラリの `analyze` を使う。

## Library

```rust
let report = nicevg::analyze(svg)?;
let result = nicevg::fix(svg)?;

println!("{:?}", report.issues);
println!("{} {:?} {:?}", result.svg, result.changes, result.report);
```

レポートと修正結果は `serde::Serialize` を実装しており、JSON へ書き出せる。

コア部分は、入力 SVG から修正後 SVG までを純粋関数のパイプラインとして構成している。

- `xml`: SVG を値としての DOM に読み書きする。更新は新しい文書を返す
- `diagram`: 文書からノード・コネクタ・ラベルを読み取る
- `inspect`: 図から問題を求める検査を順に合成する
- `route`: 辺と端点の選択、直交経路の探索、ラベルの置き場所を求める
- `fix`: ノードの拡張 → ノードの分離 → コネクタの引き直しとラベルの移動 → `viewBox` の拡張、の各段階を下書きから新しい下書きへの関数として順に適用する

## SVG contract

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
- ノード外のラベルには `data-label="label-id"` を付ける。
- コネクタの説明ラベルには `data-label-for="connector-id"` も付ける。コネクタを引き直すと、ラベルはその脇へ移動する。
- 自動経路修正の対象となるコネクタには `data-from` と `data-to` を付ける。
- コネクタのパスは `M`、`L`、`H`、`V` による折れ線を使用する。
- 現在の座標変換は `translate(...)` をサポートする。

対応できない自由形状には意味を推測せず、`unsupportedElements` としてレポートする。自動修正でもその要素は変更しない。

## Development

自然言語のテストリストは `docs/test-list/diagram.md` に置く。実装は Canon TDD の縦方向のサイクルで進める。

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check

# 修正の入出力をギャラリーで確認する（test-results/fix-gallery.html）
cargo run --example fix-gallery
```

単体テストは出力の座標を固定せず、Given/When/Then の Then が約束する性質（直交している、ノードを貫通しない、辺から垂直に出入りする など）を `tests/support/mod.rs` の述語で確かめる。

修正のテストは、Then の性質に加えて、修正後の SVG そのものと、その品質（残る問題・折れ曲がり・斜めの線分・コネクタから離れたラベルの数）を [insta](https://insta.rs) のスナップショットとして `tests/snapshots/` に記録する。個々の期待値が固定していない形の変化も、スナップショットの差分として検出される。品質の差分を見れば、その変化が改善か後退かを判断できる。改善も差分として失敗するので、意図した変化は差分を確認してから記録し直す。

```sh
cargo insta review
# または
INSTA_UPDATE=always cargo test --test fix
```

Windows では GNU ツールチェーン（`x86_64-pc-windows-gnu`）でビルドしており、依存クレートのリンクに MinGW-w64 の `dlltool` を使う。
