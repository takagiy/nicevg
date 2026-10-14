# 開発

## 作り

コア部分は、入力 SVG から修正後 SVG までを純粋関数のパイプラインとして構成している。

- `xml`: SVG を値としての DOM に読み書きする。更新は新しい文書を返す
- `diagram`: 文書からノード・コネクタ・ラベルを読み取る
- `inspect`: 図から問題を求める検査を順に合成する
- `route`: 辺と端点の選択、直交経路の探索、ラベルの置き場所を求める
- `fix`: ノードの拡張 → ノードの分離 → コネクタの引き直しとラベルの移動 → `viewBox` の拡張、の各段階を下書きから新しい下書きへの関数として順に適用する
- `arrange`: 修正結果を出発点に、ノードの小さな移動を候補ごとに修正して採点し、最も良い移動を採用することを繰り返す
- `sketch`: ノードとコネクタの配列で渡した図を、[SVG の約束](../reference/svg-contract.md) どおりの SVG に組み立てる

`nicevg` コマンドは `bindings/js/src/cli.ts` にある。標準入力の SVG を TypeScript ライブラリの `render()` に渡し、ネイティブライブラリの `arrange` で修正と配置調整をする。Rust のクレートは CLI を持たない（[ADR](../adr/cli-on-node.md)）。

## テスト

実装は Canon TDD の縦方向のサイクルで進める。自然言語のテストリストは [docs/test-list/diagram.md](../test-list/diagram.md) に置く。

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check

# 修正の入出力をギャラリーで確認する（test-results/fix-gallery.html）
cargo run --example fix-gallery
```

単体テストは出力の座標を固定せず、Given/When/Then の Then が約束する性質（直交している、ノードを貫通しない、辺から垂直に出入りする など）を `tests/support/mod.rs` の述語で確かめる。

修正のテストは、Then の性質に加えて、修正後の SVG そのものと、その品質（残る問題・折れ曲がり・斜めの線分・コネクタから離れたラベルの数・並走するコネクタの密度・ジョグ・短い端・交差・角に寄った端子・遠回り、それらを人が見比べた判断から決めた重みで合計した悪さ。どれも悪いほど大きい）を [insta](https://insta.rs) のスナップショットとして `tests/snapshots/` に記録する。個々の期待値が固定していない形の変化も、スナップショットの差分として検出される。品質の差分を見れば、その変化が改善か後退かを判断できる。改善も差分として失敗するので、意図した変化は差分を確認してから記録し直す。

```sh
cargo insta review
# または
INSTA_UPDATE=always cargo test --test fix
```

## 処理時間

処理時間は `examples/profile.rs` で測る。ライブラリの主な処理には [tracing](https://docs.rs/tracing) のスパンが付いており、スパンごとの呼び出し回数・合計時間・自分の時間（子のスパンを除いた時間）と、記録した数値（経路探索のグリッドの大きさ、探索した状態数など）を表にする。arrange の評価は並列なので、時間はスレッドの合計になる。

```sh
cargo run --release --example profile -- tests/fixtures/loan-origination-dfd.svg
# 修正だけを測り、Perfetto で開くトレースとフレームグラフ用のスタックも書き出す
cargo run --release --example profile -- input.svg --fix --chrome trace.json --folded stacks.folded
```

## CI とリリース

- CI（`.github/workflows/ci.yml`）は main への push とプルリクエストで、整形と lint と、Linux・macOS・Windows それぞれで Rust のテストと TypeScript ライブラリのビルドとテスト（Bun と Node）を回す。Rust のビルドは [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) で、Bun の依存は `bun.lock` をキーにキャッシュし、キャッシュは main の実行でだけ保存する。
- `v*` のタグを push すると、`.github/workflows/npm.yml` が npm に公開し、npm がその版を配り始めたら [takagiy/homebrew-tap](https://github.com/takagiy/homebrew-tap) の formula を npm のパッケージの tarball とその sha256 に書き換えて commit する。formula は Node でそのパッケージを入れる。tap への書き込みには、tap の contents だけに書き込める fine-grained token を `HOMEBREW_TAP_TOKEN` として secret に入れておく。
- リリースでは、ルートと `bindings/js` の `Cargo.toml`、`bindings/js/package.json` の版を揃えて上げる。

## TypeScript ライブラリ

Bun と Node（型を取り除いて .ts を動かせる 22.18 以降）、Rust のツールチェーンが要る。`bindings/js` で次を実行する。

```sh
bun install
bun run build       # cargo でネイティブライブラリを作って native/ に置き、tsc で src/ を dist/ に書き出す
bun run test        # Bun で src/ を、Node でビルドした dist/ をテストする
bun run typecheck
```

テストは `nicevg` をパッケージ名で読み込むので、型の変更は `bun run build:js` のあとでないと型検査に反映されない。

パッケージは Bun には `src/` の TypeScript を、Node には `dist/` の JavaScript と型定義を渡す（`package.json` の `exports`）。ネイティブライブラリは `NICEVG_LIBRARY` 環境変数、`native/<platform>-<arch>/`（`bun run build` が置く場所で、公開するパッケージでは全環境ぶん）、リポジトリの `target/release/` の順に探す。

`v*` のタグを push すると、`.github/workflows/npm.yml` が各環境のネイティブライブラリを作ってパッケージに入れ、Bun と Node でテストしてから npm に公開する。タグの版と `package.json` の `version` が違えば止まる。手で実行すると、公開せずにビルドとパックだけを確かめられる。

公開は npm の Trusted Publishing（GitHub Actions の OIDC）で、npm のトークンを使わず、配布元の証明（provenance）も自動で付く。npm のパッケージページは `bindings/js/README.md` で、公開した版ごとに保存されるので、README の変更は次の公開で反映される。

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

## Windows

Windows では GNU ツールチェーン（`x86_64-pc-windows-gnu`）でビルドしており、依存クレートのリンクに MinGW-w64 の `dlltool` を使う。
