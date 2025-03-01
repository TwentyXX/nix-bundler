use anyhow::{Context as _, anyhow};
use clap::{Parser, Subcommand, command};
use nix_bundler::bundle_nix_files;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// バンドルモード：複数のNixファイルを1つのファイルにバンドルします
    Bundle {
        /// エントリーポイントとなるNixファイル
        #[arg(short, long)]
        entry: PathBuf,

        /// 出力ファイル名
        #[arg(short, long, default_value = "bundled.nix")]
        output: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Bundle { entry, output } => {
            println!("エントリーポイント: {}", entry.display());
            println!("出力ファイル: {}", output.display());

            // エントリーポイントが存在するか確認
            if !entry.exists() {
                return Err(anyhow!(
                    "エントリーポイントファイルが存在しません: {}",
                    entry.display()
                ));
            }

            // 依存関係グラフを構築
            let bundled_content = bundle_nix_files(entry)?;

            // 結果を出力ファイルに書き込む
            fs::write(output, bundled_content)?;

            println!("バンドル完了: {}", output.display());

            // nix-instantiateで検証
            validate_with_nix_instantiate(output)?;

            Ok(())
        }
    }
}

/// nix-instantiateでバンドルされたファイルを検証する関数
fn validate_with_nix_instantiate(output_file: &Path) -> anyhow::Result<()> {
    println!("nix-instantiateで検証しています...");

    let output = Command::new("nix-instantiate")
        .arg("--eval")
        .arg(output_file)
        .output()
        .with_context(|| "nix-instantiateの実行に失敗しました")?;

    if output.status.success() {
        println!("検証成功: バンドルされたファイルは有効なNix式です");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("検証失敗: {}", error))
    }
}
