# 一対比較の問題と回答（realistic2-128）

品質指標の重み（`tests/support/mod.rs` の `BADNESS_WEIGHTS`）を決めるために、人が「どちらの図の方が悪く見えるか」を答えた記録。経緯と分析は docs/idea/quality-weights.md にある。

## ファイル

- `bases/<図>.svg`: 元の図。4 つの例の図（storefront、order、c4、loan）を arrange した出力。
- `questions.json`: 問題。
  - `variants`: 元の図を崩した版。`base` が元の図、`ops` が元の図からの行の差分、`delta` が元の図からの品質指標の差（悪化が正）、`mark` が変えた場所を囲む枠（SVG 座標）。`kind` は作り方で、`route` は見えない障害物を置いて fix で引き直させ、障害物を除いたもの、`port` は端を辺に沿ってずらしたもの。`main` が真の版は本編に出る。
  - `questions`: 本編 128 問。`a` と `b` が比べる版。
  - `followups`: 本編のあとにページが 8 問ずつ作った追加 64 問。`reason` が `info` なら重みが定まっていない方向を詰める組、`recheck` なら `of` の問題を左右を入れ替えて聞き直したもの。
  - `features`: 指標の名前。
- `answers.json`: 回答（回答者 1 人）。`choice` は -3（A が悪い）、-2（どちらかといえば A）、-1（ほぼ差はないが A）、1、2、3（B が悪い）。

## 版の SVG を作り直す

`ops` は `[from, to, lines]` の並びで、元の図を改行で分けた行の `from` から `to` の手前までを `lines` に置き換える。後ろの操作から順に当てる。

```python
lines = base_svg.split('\n')
for start, end, replacement in reversed(variant['ops']):
    lines[start:end] = replacement
svg = '\n'.join(lines)
```

どの版も、元の図にない問題を検査で出さず、表示範囲とノードは元の図のまま、変わった線の端は 8px 以上ある。
