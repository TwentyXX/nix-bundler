use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use path_clean::clean;


/// Nixファイルのインポート情報を表す構造体
#[derive(Clone)]
struct NixImport {
    /// インポートパス（相対パスまたは絶対パス）
    path: PathBuf,
    /// ソースコード内での位置（行、列）
    _position: (usize, usize),
    /// インポート文の全体（置換用）
    full_import: String,
}

/// Nixファイルの解析結果を表す構造体
#[derive(Clone)]
struct NixFile {
    /// ファイルパス
    path: PathBuf,
    /// ファイルの内容
    content: String,
    /// インポート情報のリスト
    imports: Vec<NixImport>,
}


/// Nixファイルをバンドルする関数
pub fn bundle_nix_files(entry_point: &Path) -> Result<String> {
    // 処理済みファイルを追跡するためのセット
    let mut processed_files = HashSet::new();
    // ファイルパスとその内容のマップ
    let mut file_contents = HashMap::new();
    
    // エントリーポイントから再帰的に依存関係を解析
    process_nix_file(entry_point, &mut processed_files, &mut file_contents)?;
    
    // エントリーポイントの絶対パスを取得
    let abs_entry_path = if entry_point.is_absolute() {
        entry_point.to_path_buf()
    } else {
        std::env::current_dir()?.join(entry_point)
    };
    
    // クリーンなパスに変換
    let clean_entry_path = PathBuf::from(clean(abs_entry_path.to_string_lossy().as_ref()));
    
    // エントリーポイントが存在するか確認
    if !file_contents.contains_key(&clean_entry_path) {
        return Err(anyhow!("エントリーポイントの内容が見つかりません: {}", clean_entry_path.display()));
    }
    
    // インライン化された内容を生成
    let bundled_content = inline_imports(&clean_entry_path, &file_contents)?;
    
    Ok(bundled_content)
}

/// Nixファイルを解析して依存関係を処理する関数
fn process_nix_file(
    file_path: &Path,
    processed_files: &mut HashSet<PathBuf>,
    file_contents: &mut HashMap<PathBuf, NixFile>
) -> Result<()> {
    // 絶対パスに変換
    let abs_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(file_path)
    };
    
    // クリーンなパスに変換
    let clean_path = PathBuf::from(clean(abs_path.to_string_lossy().as_ref()));
    
    // すでに処理済みの場合はスキップ
    if processed_files.contains(&clean_path) {
        return Ok(());
    }
    
    // ファイルが存在するか確認
    if !clean_path.exists() {
        return Err(anyhow!("ファイルが存在しません: {}", clean_path.display()));
    }
    
    // ファイルの内容を読み込む
    let content = fs::read_to_string(&clean_path)
        .with_context(|| format!("ファイルの読み込みに失敗しました: {}", clean_path.display()))?;
    
    // インポート文を解析
    let imports = parse_imports(&content, &clean_path)?;
    
    // ファイル情報を保存
    let nix_file = NixFile {
        path: clean_path.clone(),
        content: content.clone(),
        imports: imports.clone(),
    };
    file_contents.insert(clean_path.clone(), nix_file);
    
    // 処理済みとしてマーク
    processed_files.insert(clean_path.clone());
    
    // 依存ファイルを再帰的に処理
    for import in imports {
        let import_path = resolve_import_path(&import.path, &clean_path)?;
        process_nix_file(&import_path, processed_files, file_contents)?;
    }
    
    Ok(())
}

/// インポートパスを解決する関数
fn resolve_import_path(import_path: &Path, current_file: &Path) -> Result<PathBuf> {
    // インポートパスが絶対パスの場合はそのまま返す
    if import_path.is_absolute() {
        return Ok(import_path.to_path_buf());
    }
    
    // 相対パスの場合は、現在のファイルのディレクトリを基準に解決
    let parent_dir = current_file.parent()
        .ok_or_else(|| anyhow!("親ディレクトリが見つかりません: {}", current_file.display()))?;
    
    let resolved_path = parent_dir.join(import_path);
    let clean_resolved_path = PathBuf::from(clean(resolved_path.to_string_lossy().as_ref()));
    
    Ok(clean_resolved_path)
}

/// Nixファイル内のインポート文を解析する関数
fn parse_imports(content: &str, file_path: &Path) -> Result<Vec<NixImport>> {
    let mut imports = Vec::new();
    
    // importステートメントを検出する正規表現
    // 注意: これは簡易的な実装で、すべてのケースをカバーしていない可能性があります
    let import_regex = Regex::new(r#"import\s+(?:(?:"([^"]+)")|(?:'([^']+)')|([^\s;]+))"#)?;
    
    // 各行を処理
    for (line_idx, line) in content.lines().enumerate() {
        for captures in import_regex.captures_iter(line) {
            let path_str = captures.get(1).or_else(|| captures.get(2)).or_else(|| captures.get(3))
                .ok_or_else(|| anyhow!("インポートパスが見つかりません: {}:{}", file_path.display(), line_idx + 1))?
                .as_str();
            
            let full_import = captures.get(0).unwrap().as_str().to_string();
            let column = captures.get(0).unwrap().start();
            
            let import_path = PathBuf::from(path_str);
            
            imports.push(NixImport {
                path: import_path,
                _position: (line_idx + 1, column),
                full_import,
            });
        }
    }
    
    Ok(imports)
}

/// インポートをインライン化する関数
fn inline_imports(
    entry_point: &Path,
    file_contents: &HashMap<PathBuf, NixFile>
) -> Result<String> {
    // インライン化済みファイルを追跡
    let mut inlined_files = HashSet::new();
    
    // 再帰的にインライン化
    inline_file_recursive(entry_point, file_contents, &mut inlined_files)
}

/// ファイルを再帰的にインライン化する関数
fn inline_file_recursive(
    file_path: &Path,
    file_contents: &HashMap<PathBuf, NixFile>,
    inlined_files: &mut HashSet<PathBuf>
) -> Result<String> {
    // 絶対パスに変換
    let abs_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(file_path)
    };
    
    // クリーンなパスに変換
    let clean_path = PathBuf::from(clean(abs_path.to_string_lossy().as_ref()));
    
    // ファイル情報を取得
    let nix_file = file_contents.get(&clean_path)
        .ok_or_else(|| anyhow!("ファイル情報が見つかりません: {}", clean_path.display()))?;
    
    // すでにインライン化済みの場合は空文字列を返す（循環参照を防ぐ）
    if inlined_files.contains(&clean_path) {
        return Ok(String::new());
    }
    
    // インライン化済みとしてマーク
    inlined_files.insert(clean_path.clone());
    
    let mut result = nix_file.content.clone();
    
    // インポートを逆順に処理（テキスト位置が変わらないように）
    for import in nix_file.imports.iter().rev() {
        let import_path = resolve_import_path(&import.path, &clean_path)?;
        
        // インポートファイルをインライン化
        let inlined_content = inline_file_recursive(&import_path, file_contents, inlined_files)?;
        
        // インポート文を置換
        // 注意: これは簡易的な実装で、複雑なケースでは問題が発生する可能性があります
        result = result.replace(&import.full_import, &inlined_content);
    }
    
    // インライン化済みとしてマークを解除（他のパスからの参照のため）
    inlined_files.remove(&clean_path);
    
    Ok(result)
}
