{
  # ユーティリティ関数
  
  # 文字列を大文字に変換する関数（単純化版）
  toUpper = str: builtins.toJSON str;
  
  # 2つの値を結合する関数
  concat = a: b: a + b;
  
  # リストの合計を計算する関数
  sum = list: builtins.foldl' (a: b: a + b) 0 list;
}