let
  # 設定ファイルをインポート
  config = {
  # 設定値
  name = "example";
  version = "1.0.0";
  
  # システム設定
  system = {
    arch = "x86_64";
    os = "linux";
  };
};
  
  # ライブラリをインポート
  lib = {
  # ユーティリティ関数
  
  # 文字列を大文字に変換する関数（単純化版）
  toUpper = str: builtins.toJSON str;
  
  # 2つの値を結合する関数
  concat = a: b: a + b;
  
  # リストの合計を計算する関数
  sum = list: builtins.foldl' (a: b: a + b) 0 list;
};
in
{
  # メインの出力
  name = lib.toUpper config.name;
  version = config.version;
  
  # システム情報
  system = config.system;
  
  # 計算結果
  calculated = {
    nameWithVersion = lib.concat config.name "-" + config.version;
    numbers = lib.sum [ 1 2 3 4 5 ];
  };
}