let
  # 設定ファイルをインポート
  config = import ./config.nix;
  
  # ライブラリをインポート
  lib = import ./lib.nix;
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