このドキュメントは [English](GUIDE.md) | [한국어](GUIDE.ko.md) | **日本語** でも利用可能です。

# LUX (Linalab Unity X) 開発者ガイド

LUXは、UnityエディターとAIコーディングツール（Claude Code、OpenAI Codex、OpenCodeなど）を接続する統合アダプターおよび自動化ツールキットです。このガイドは、LUXの構造を理解し活用しようとする開発者のために作成されました。

## 1. 紹介

LUXは、Unityエディター内部の作業を外部のAIツールが制御できるようにブリッジの役割を果たします。単なるコマンドの伝達を超え、ウェブベースのコントロールサーフェス、視覚的なパイプラインエディター、WebRTCを利用した遠隔制御機能を提供し、Unity開発環境の自動化レベルを高めます。

## 2. インストール

### Unityパッケージのインストール
1. Unityプロジェクトの`Packages/manifest.json`にLUXパッケージを追加します。
2. `com.unity.webrtc`パッケージがインストールされていることを確認してください（遠隔ストリーミング機能に必要です）。

### Rust CLIのインストール
LUXのゲートウェイとCLIツールはRustで作成されています。
```bash
cd Packages/com.linalab.lux/RustGateway~
cargo build --release
# ビルドされた実行ファイルをPATHに登録するか、直接実行します。
./target/release/lux --help
```

## 3. クイックスタート

5分以内にLUXサーバーを起動し、Unityと接続する方法です。

1. **Unityエディターの実行**: プロジェクトを開き、`Window > Linalab > Lux Workbench`を開きます。
2. **サーバーの実行**: ターミナルで次のコマンドを入力します。
   ```bash
   # 基本実行（30分間アイドル状態で自動終了）
   lux serve --port 8080

   # アイドルタイムアウトの変更（0 = 無効化）
   lux serve --port 8080 --idle-timeout 60
   ```
3. **ウェブUIへのアクセス**: ブラウザで`http://localhost:8080`にアクセスします。
4. **接続の確認**: `Tools > Linalab > Lux > Server Status`ウィンドウでサーバーの状態を確認します。緑色なら接続済み、黄色ならサーバー未実行、赤色ならエラーです。
5. **サーバーのライフサイクル**: サーバーはUnityエディターがアクティブな間、実行を維持します。30分間活動がないと自動的に終了します（`--idle-timeout`で調整可能）。

## 4. アーキテクチャ

LUXはいくつかの主要モジュールで構成されています。

| モジュール | 説明 |
| :--- | :--- |
| **LuxEditor** | メインアダプター。ワークベンチウィンドウ、自動化ゲートウェイ、WebRTCプロデューサーを含む。 |
| **AiBridgeEditor** | AIツールとの通信のためのTCPサーバーおよびプロトコルハンドラー。 |
| **UnityGitEditor** | Unity内部でのGit状態確認、ステージング、ブランチ管理をサポート。 |
| **CodexImage** | ノードベースの画像生成パイプラインエンジン。 |
| **RustGateway** | AxumベースのウェブサーバーおよびCLI。ウェブUIとAPIエンドポイントを提供。 |
| **Skills** | Unity制御のためのコアスキルセットおよび参照ドキュメント。 |

## 5. CLIリファレンス

`lux`コマンドラインツールを通じて、サーバー管理およびUnityの制御が可能です。

| コマンド | 説明 |
| :--- | :--- |
| `lux serve` | ウェブサーバーおよびゲートウェイの実行。 |
| `lux compile` | Unityプロジェクトのコンパイル実行。 |
| `lux test` | プレイモードおよびエディットモードのテスト実行。 |
| `lux unity status` | Unityエディターの接続状態の確認。 |
| `lux unity screenshot` | 現在のエディター画面のキャプチャ。 |
| `lux unity logs` | Unityコンソールログのストリーミング。 |
| `lux unity dynamic-code` | Unity内部でのC#コードの動的実行。 |
| `lux skill list` | インストールされたスキル一覧の確認。 |
| `lux skill install <name>` | 新しいスキルのインストール。 |

## 6. ウェブUI

ゲートウェイサーバーの実行後、ブラウザを通じて次の機能を使用できます。

- **AIターミナル (AITerminal)**: Claude、Codexなど様々なAIツールを切り替えて使用。
- **パイプラインエディター (NodeEditor)**: ReactFlowベースの視覚的ツールで画像生成ワークフローを設計。
- **遠隔ビューアー (RemoteViewer)**: WebRTCを通じてUnity画面をリアルタイムで確認し、マウス/キーボード入力を伝達。
- **セッションマネージャー**: 現在アクティブなAIツールのセッションおよびコマンド履歴の管理。

## 7. スキルシステム

スキルは、AIがUnityを制御する方法を定義した単位です。

- **コアスキル**: `lux-unity`スキルが標準で含まれており、コンパイル、テスト、ログ確認などをサポートします。
- **スキル管理**:
  ```bash
  # スキル情報の確認
  lux skill info lux-unity
  # 外部スキルのインストール
  lux skill install my-custom-skill --source https://github.com/user/repo
  ```

## 8. APIリファレンス

外部ツールとの連携のための主要なエンドポイントです。

| エンドポイント | メソッド | 説明 |
| :--- | :--- | :--- |
| `/health` | GET | サーバー状態およびプロトコルバージョンの確認。 |
| `/api/health` | GET | サーバーの稼働時間（uptime）および状態レポート。 |
| `/api/heartbeat` | POST | Unityエディターから定期的に呼び出し、アイドルタイマーを更新。`{ "status": "alive", "uptime_seconds": N }`を返却。 |
| `/api/sessions` | GET/POST | AIツールセッションの管理。 |
| `/api/graphs` | GET/POST | パイプライングラフの保存および読み込み。 |
| `/api/tools/execute` | POST | 特定のAIツールにコマンドを伝達。 |
| `/api/remote/signaling` | POST | WebRTCシグナリングデータの交換。 |
| `/events` | WS | リアルタイムイベントストリーミング (WebSocket)。 |

## 9. 遠隔接続 (WebRTC)

LUXはUnityの画面をウェブブラウザにストリーミングします。

- **設定**: UnityエディターのLux Workbenchで解像度とフレームレートを調整できます。
- **ネットワーク**: ローカルネットワークの外部から接続するには、STUN/TURNサーバーの設定が必要です。ゲートウェイの設定ファイルでICEサーバー情報を入力してください。

## 10. 開発ガイド

### テストの実行
- **Rust**: `cargo test`（ユニットテストおよびスモークテストを含む）
- **C#**: Unity Test Runnerで`AiBridgeTests`、`LuxTests`などを実行します。

### 貢献方法
1. 新しい機能を追加する際は、`LuxEditor`モジュールのゲートウェイポリシーをまず確認してください。
2. ウェブUIの修正時は、`RustGateway~/ui-src`パスのReactコンポーネントを修正します。
3. 変更事項の適用後は、必ず`lux test`を通じて回帰テストを実行してください。

## 11. トラブルシューティング

- **接続失敗**: Unityエディターが実行中か、AI Bridge TCPサーバーが有効になっているかを確認してください。
- **サーバーが頻繁に終了する**: `--idle-timeout 0`でアイドルタイムアウトを無効にするか、UnityエディターでServer Statusウィンドウが開いているかを確認してください（60秒ごとにハートビートを送信します）。
- **WebRTCの画面が表示されない**: `com.unity.webrtc`パッケージのバージョンの互換性を確認し、ブラウザのコンソールログでシグナリングエラーをチェックしてください。
- **権限エラー**: 自動化コマンドの実行時に、Unityエディターで承認ポップアップが表示されていないか確認してください。
- **TypeScriptエラー**: `cd RustGateway~/ui-src && npx tsc --noEmit`で確認してください。strictモードが有効になっているため、型エラーは必ず修正する必要があります。
