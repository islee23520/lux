このドキュメントは [English](AGENTS.md) | [한국어](AGENTS.ko.md) | **日本語** でも利用可能です。

# LUX (Linalab Unity X) エージェントガイド

LUXは、統合されたUnityエディターAIアダプターおよび自動化ツールキットです。これは独立したUnityパッケージであり、スタンドアロンのアプリケーションではありません。

## コードベースの構造

| パス | 説明 | アセンブリ / 技術 |
| :--- | :--- | :--- |
| `LuxEditor/` | UnityエディターC#スクリプト | `Linalab.LuxEditor` |
| `AiBridgeEditor/` | TCPサーバーおよびプロトコル | `Linalab.UnityAiBridge.Editor` |
| `UnityGitEditor/` | Git統合 | `Linalab.UnityGit.Editor` |
| `CodexImage/` | 画像生成パイプライン | C#エディタースクリプト |
| `RustGateway~/` | Rust CLIおよびWebサーバー | Axum 0.7, React 19 |
| `McpHelper~/` | Node.js MCPヘルパー | Node.js |
| `Skills/lux-unity/` | コアAIスキル | Manifest + SKILL.md |
| `*Tests/` | C#およびRustテストスイート | NUnit / Cargo |

## 主要な規約

### Rust (`RustGateway~/`)
- Axum 0.7、tokio 1、clap 4.5、anyhow、serdeを使用してください。
- エラーハンドリング: ロジックには`anyhow`を、ユーザー出力には`eprintln`を使用してください。
- `TODO`、`FIXME`、`HACK`コメントは禁止です。
- 新しいエンドポイントには、`server.rs`または`gateway_cli_smoke.rs`にテストを含める必要があります。
- サーバーのライフサイクル: アイドルタイムアウトによる正常終了（`--idle-timeout`）、ハートビート（`POST /api/heartbeat`）、ヘルスチェック（`GET /api/health`）。

### TypeScript (`RustGateway~/ui-src/`)
- React 19とTypeScriptのstrictモードを使用してください。
- 関数コンポーネントとフックを使用してください。
- APIフックにモックデータやフォールバックデータを含めないでください。
- 状態管理: `useState`、`useRef`、`useCallback`、`useEffect`を使用してください。

### C# (Editorディレクトリ)
- 名前空間: `UnityEditor`。アセンブリ: `Linalab.LuxEditor`。
- すべてのクラスに`Lux`プレフィックスを付けてください。
- ロジックをグループ化するために、大きなファイルにはpartialクラスを使用してください。
- 巨大なC#ファイルはpartialクラスで分割されています（例: LuxAutomationGatewayは約10ファイル、LuxWebRTCProducerは約7ファイルに分割）。
- テスト: `*Tests/Editor/`ディレクトリでNUnitの`[Test]`を使用してください。

### スキル (Skills)
- コアスキルは`Skills/`にあります。これらは削除できません。
- 構造: `manifest.json`、`SKILL.md`、`references/`。

## アンチパターン (禁止事項)
- C#クラス名から`Lux`プレフィックスを削除しないでください。
- APIフックにモックデータやフォールバックデータを入れたりしないでください。
- TypeScriptのstrictモードを無効にしないでください。
- CLIのコアスキル保護を削除しないでください。
- `cargo test`を実行せずにコミットしないでください。
- テストをパスさせるためだけにテストファイルを編集しないでください。
- ホストプロジェクト（neon-glitch）をLUXの一部として扱わないでください。

## 検証コマンド

### Rust
```bash
cd RustGateway~ && cargo build && cargo test
```

### TypeScript
```bash
cd RustGateway~/ui-src && npx tsc --noEmit
```

### CLIヘルプ
```bash
cd RustGateway~ && cargo run -- skill install --help
cd RustGateway~ && cargo run -- serve --help
```

### C#
LSP診断を使用して検証してください。CLIビルドコマンドは提供されていません。
