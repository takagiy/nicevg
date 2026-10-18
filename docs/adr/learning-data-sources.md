# 学習用の図は生成器で作り、公開データは BPMAI と SchemaPile-Perm だけを使う

## 状況

ふるい分けのモデル（docs/adr/screen-moves-with-learning.md）を学ぶには、arrange を回して候補を記録する図が数千枚要る。手元の例の図は 6 つしかない。

nicevg に渡すには、ノードの位置と大きさ、つながり、ラベルが要る。公開されている図やグラフのデータで位置まで持つものは少なく、多くはつながりしか持たない。また、学習したモデルを MIT の npm パッケージに入れて配るので、使えるのは商用で使えて派生物を作れるライセンスのものに限られる。

調べた公開データ:

| データ | 中身 | 位置 | ライセンス |
|---|---|---|---|
| [BPMAI のモデル集](https://zenodo.org/records/3758705)（2019-10） | 約 3 万の業務プロセス図（主に BPMN）。Signavio の JSON、圧縮して 388MB | あり（人が描いた図形の範囲と、線の折れ点） | CC BY 3.0 |
| [SAP-SAM](https://zenodo.org/record/7012043) | 約 60 万の業務プロセス図。BPMAI の続き、圧縮して 4.8GB | あり | 非営利の研究にだけ使える。派生物を作る権利はない |
| [SchemaPile-Perm](https://github.com/amsterdata/schemapile)（[Zenodo](https://zenodo.org/records/10931803)） | GitHub の SQL から集めた、許可の緩いライセンスのデータベースのスキーマ。全体の SchemaPile は 22 万件、テーブル 170 万、外部キー 70 万。Perm は JSON で 27MB | なし | CC BY 4.0（元の SQL は MIT などの緩いライセンスのものだけ） |
| [ELK のテストモデル](https://github.com/eclipse/elk-models) | ELK のテスト用のグラフ。`realworld/ptolemy` に実際の図から変換したもの約 1,650 件 | 一部あり（ELK の形式） | EPL-2.0 |
| [Rome グラフ・AT&T グラフ](https://graphdrawing.unipg.it/data.html) | Rome はノード 10〜100 の小さなグラフ 11,528 個。AT&T は実際のグラフ 5,000 余り | なし | 明記されたライセンスは確認できていない |
| [ModelSet](https://github.com/modelset) / [Lindholmen](https://link.springer.com/doi/10.1007/s10270-021-00929-3) | UML と Ecore のモデル。ModelSet は 5,466 の Ecore と 5,120 の UML、Lindholmen は GitHub の UML 約 10 万 | 一部（図の情報付きの XMI だけ） | ModelSet は配布元のライセンスに従う。Lindholmen は元のリポジトリごとにまちまち |
| Hugging Face の Mermaid のデータ（[Flowchart2Mermaid](https://huggingface.co/datasets/MananSuri27/Flowchart2Mermaid)、[FlowGen](https://huggingface.co/datasets/Anonymous112233/FlowGen/blob/main/README.md) など） | フローチャートの画像と Mermaid のコードの組。数百〜数千件 | なし | データごとに違う |

## 決定

- 学習用の図の中心は、乱数で図を作る生成器（`examples/generate.rs`）にする。乱数の種を固定して、DFD 風、C4 風、ER 風、フロー図、構成図を作る。公開データでは足りない図の大きさや種類の偏りを、生成器で埋める。
- 公開データは BPMAI と SchemaPile-Perm を使う。
  - BPMAI は、人が描いた配置を持つ唯一の大きなデータ。DFD の例に近く、生成器より実際の図に近い分布が手に入る。図形の種類（タスク、ゲートウェイ、イベント）は、nicevg の矩形と円にそのまま写せる。
  - SchemaPile-Perm は ER 図に向く。テーブルを列のポート付きのノードに、外部キーをポートどうしの線にする。配置がないので、格子か ELK の layered で並べて下書きにする。
  - どちらも CC BY なので、モデルの置き場所に出典を書く。
- 例の 6 つの図は学習に入れず、確かめるためだけに使う。

## 結果

得たもの:

- MIT のパッケージにモデルを入れて配れる。
- 生成器 720 枚、BPMAI 1,375 枚、SchemaPile 600 枚の約 2,700 枚、約 156 万件の候補で最初の学習ができた。

失ったもの:

- 位置のないデータを Graphviz や ELK で並べると、下書きが整いすぎて、arrange のすることが少なくなりうる。手で描いた図のずれを再現するため、並べたあとに位置を少し乱数で崩す必要がある。
- 業務プロセス図の量では、SAP-SAM の 60 万を使えず、BPMAI の 3 万にとどまる。

## 採らなかった案

- **SAP-SAM:** 非営利の研究にしか使えず、派生物を作る権利がない。学習したモデルを配れないので、手元で確かめる用途にも使わない。
- **ELK のテストモデル:** ライセンス（EPL-2.0）は条件に合い、ポート付きのブロック図に向くが、まだ使っていない。
- **Rome グラフ・AT&T グラフ:** 研究用に長く使われてきたが、明記されたライセンスを確認できない。ラベルや大きさもない。
- **ModelSet / Lindholmen:** ライセンスを 1 件ずつ確かめる手間が大きい。
- **Mermaid のデータ:** 小さく、合成のものも多いので、生成器の代わりにならない。

経緯は docs/idea/learned-guidance.md にある。
