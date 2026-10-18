// scikit-learn mixed-type prediction pipeline.
// Sources: https://scikit-learn.org/stable/auto_examples/compose/plot_column_transformer_mixed_types.html
// and https://scikit-learn.org/stable/modules/compose.html
// Follows Titanic feature rows through dtype-based dispatch, separate numeric and categorical
// preprocessing, feature concatenation and LogisticRegression prediction.

import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "x-test", class: "box", x: 30, y: 240, label: "X_test DataFrame\nembarked, sex, pclass, age, fare", width: 240, height: 76 },
    { id: "pipeline", class: "box", x: 320, y: 240, label: "Pipeline.predict(X_test)\npreprocessor → classifier", width: 235, height: 76 },
    { id: "column-transformer", class: "box", x: 620, y: 240, label: "ColumnTransformer\ntransform named column subsets", width: 235, height: 76 },
    { id: "num-selector", class: "box", x: 900, y: 70, label: "make_column_selector\ndtype_exclude='category'", width: 235, height: 76 },
    { id: "numeric-imputer", class: "box", x: 1190, y: 45, label: "SimpleImputer\nstrategy='median'", width: 215, height: 76 },
    { id: "scaler", class: "box", x: 1470, y: 45, label: "StandardScaler\nfit statistics from training", width: 230, height: 76 },
    { id: "cat-selector", class: "box", x: 900, y: 410, label: "make_column_selector\ndtype_include='category'", width: 235, height: 76 },
    { id: "categorical-imputer", class: "box", x: 1190, y: 355, label: "SimpleImputer\nstrategy='most_frequent'", width: 225, height: 76 },
    { id: "one-hot", class: "box", x: 1470, y: 325, label: "OneHotEncoder\nunknown categories ignored", width: 230, height: 76 },
    { id: "select-percentile", class: "box", x: 1470, y: 465, label: "SelectPercentile(chi2)\npercentile=50", width: 230, height: 76 },
    { id: "features", class: "box", x: 1770, y: 240, label: "ColumnTransformer output\nconcatenate in transformer order", width: 245, height: 76 },
    { id: "classifier", class: "box", x: 2080, y: 240, label: "LogisticRegression\npredict transformed rows", width: 235, height: 76 },
    { id: "prediction", class: "box", x: 2380, y: 240, label: "y_pred\nclass label per input row", width: 220, height: 76 },
  ] satisfies Node[],
  edges: [
    { from: "x-test", to: "pipeline", label: "DataFrame rows" },
    { from: "pipeline", to: "column-transformer", label: "X passed to each transform" },
    { from: "column-transformer", to: "num-selector", label: "select numeric columns" },
    { from: "column-transformer", to: "cat-selector", label: "select category columns" },
    { from: "num-selector", to: "numeric-imputer", label: "pclass, age, fare" },
    { from: "numeric-imputer", to: "scaler", label: "median-filled numeric values" },
    { from: "scaler", to: "features", label: "scaled numeric matrix" },
    { from: "cat-selector", to: "categorical-imputer", label: "embarked, sex" },
    { from: "categorical-imputer", to: "one-hot", label: "filled category values" },
    { from: "one-hot", to: "select-percentile", label: "one-hot feature matrix" },
    { from: "select-percentile", to: "features", label: "selected categorical features" },
    { from: "features", to: "classifier", label: "combined feature matrix" },
    { from: "classifier", to: "prediction", label: "predicted class labels" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
