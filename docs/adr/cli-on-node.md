# CLI は TypeScript ライブラリの bin にし、npm で配る

## 状況

CLI は Rust のバイナリ（`src/main.rs`）で、`brew` か `cargo install --git` で入れるしかなかった。TypeScript ライブラリはすでに npm で、全環境向けのビルド済みネイティブライブラリを持っている（[ADR](typescript-through-native-ffi.md)）。エージェントの動く環境には Node か Bun があることが多く、Rust のツールチェーンはないことが多い。CLI の振る舞い（標準入力の SVG を修正と配置調整して標準出力へ、残った問題は標準エラーへ、終了コード 0・1・2、オプションは `--help` と `--version` だけ）は保ちたい。

## 決定

- CLI を `bindings/js/src/cli.ts` に書き直し、`package.json` の `bin` で `nicevg` コマンドにする。`npm install -g nicevg`、`npx nicevg`、`bunx nicevg` で動く。
- 処理は今までどおり Rust のネイティブコードで、CLI は `render({ svg })` を呼ぶだけにする。これは Rust の CLI と同じ `nicevg::arrange` を呼ぶ。
- Rust の CLI（`src/main.rs`）と `tests/cli.rs` を消す。CLI のテストは同じ Given/When/Then のまま `bindings/js/test/node/cli.test.ts` に移し、Node（ビルドした `dist/cli.js`）と Bun（`src/cli.ts`）の両方で回す。`clap` と `mimalloc` は `examples/profile.rs` だけが使うので開発用の依存にする。
- 配置調整は、古い候補を捨てるスレッドを待ってから戻る。Rust の CLI ではプロセスごと終わるので問題にならなかったが、Node は終わる時にネイティブライブラリを外すので、捨てている途中のスレッドが落ちていた（`before.svg` を 8 並列で 160 回動かして 59 回落ちた。待つようにしてからは 480 回で 0 回）。出力は変わらない。
- Windows（MSVC でのビルド）では、ネイティブライブラリを最初の呼び出しでメモリに固定し（`GetModuleHandleExW` の PIN）、プロセスが終わるまで外させない。koffi は Node の終了時にライブラリを外すが、仕事を終えたスレッドも終わる処理の中でライブラリのコードを動かしており、外れた後にそのコードへ戻って落ちていた（GitHub Actions の Windows で CLI のテストを 20 回回して 19〜20 回落ち、mimalloc の有無によらなかった。固定してからは 0 回）。Bun はライブラリを外さないので落ちなかった。
- Homebrew の formula は、npm のパッケージの tarball を Node で入れる形にする。tap の更新は `npm.yml` の公開のあとに、npm がその版を配り始めてから行う。

## 結果

得たもの:

- Node か Bun があれば、Rust なしで CLI を入れられる。npm のパッケージ 1 つで、ライブラリと CLI の両方が手に入る。
- 出力は Rust の CLI とバイト単位で同じ（4 つの例の図と README の例で、標準出力・標準エラー・終了コードを比べた）。
- CLI とライブラリが同じネイティブライブラリを通るので、CLI だけのビルドと配布がなくなった。

失ったもの:

- CLI を動かすのに Node 20 以降か Bun が要る。Homebrew では Node が依存に入る。
- `cargo install --git` では CLI を入れられなくなった。Rust のクレートはライブラリだけになる。
- 起動のたびに Node とネイティブライブラリを読み込むので、小さな図では Rust のバイナリより遅い（何も直さない図で、Rust 38ms に対して Node・Bun とも 94ms）。
- 配るのは npm のパッケージと同じ 5 つの環境だけ。

## 採らなかった案

- Rust の CLI を残し、npm にも別に CLI を置く: 2 つの CLI の振る舞いを揃え続けることになる。
- npm のパッケージに Rust の CLI のバイナリを入れ、`bin` から起動する（esbuild などのやり方）: 環境ごとに実行ファイルも配ることになり、パッケージがさらに大きくなる。処理はすでにネイティブライブラリにあるので、起動するだけのバイナリは要らない。
