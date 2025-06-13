# Slack勤怠管理Lambda (cargo-lambda版)

slack-attendance-lambdaは、SlackとNotionを連携させた勤怠管理システムです。cargo-lambdaを使用してRustで実装されたAWS Lambda関数です。

## 機能

- Slackのスラッシュコマンド（`/attendance`）で勤怠記録
- 出勤、休憩開始、休憩終了、退勤の記録
- Notionデータベースへの自動保存
- 月次レポート機能（`/attendance report`）
- Slack署名検証によるセキュリティ確保

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)
- AWS CLI設定済み
- Slackアプリ作成済み
- Notionデータベース作成済み

## ビルド

本番環境用にビルドするには `cargo lambda build --release` を実行してください。開発用には `--release` フラグを外してください。

```bash
# 開発用ビルド
cargo build

# 本番用ビルド
cargo lambda build --release
```

詳細については [Cargo Lambda ドキュメント](https://www.cargo-lambda.info/commands/build.html) を参照してください。

## テスト

通常のRustユニットテストは `cargo test` で実行できます。

ローカルで統合テストを実行するには、`cargo lambda watch` と `cargo lambda invoke` コマンドを使用します。

### ローカルでのテスト実行

```bash
# ローカルサーバーの起動（ファイル変更時に自動再起動）
cargo lambda watch

# 別ターミナルでテストイベントによる実行
cargo lambda invoke --data-file test-event.json

# cURLでHTTPリクエストのテスト
curl -X POST http://localhost:9000/lambda-url/slack-attendance-lambda/ \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -H "X-Slack-Signature: v0=test" \
  -H "X-Slack-Request-Timestamp: $(date +%s)" \
  -d "text=in&user_id=U1234&user_name=testuser"
```

### テストデータの例

`test-event.json` ファイルでSlackからのイベントをシミュレート：

```json
{
  "version": "2.0",
  "routeKey": "POST /",
  "rawPath": "/",
  "headers": {
    "x-slack-signature": "v0=test",
    "x-slack-request-timestamp": "1234567890",
    "content-type": "application/x-www-form-urlencoded"
  },
  "body": "text=in&user_id=U1234&user_name=testuser",
  "isBase64Encoded": false
}
```

詳細については以下を参照：
- [watch コマンド](https://www.cargo-lambda.info/commands/watch.html)
- [invoke コマンド](https://www.cargo-lambda.info/commands/invoke.html)

## デプロイガイド

### 事前準備

#### 1. AWS CLIの設定
```bash
# AWS CLIのインストール（未インストールの場合）
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# 認証情報の設定
aws configure
# AWS Access Key ID: [アクセスキー]
# AWS Secret Access Key: [シークレットアクセスキー]
# Default region name: ap-northeast-1
# Default output format: json
```

#### 2. IAMロールの作成
AWS Consoleまたは以下のCLIコマンドでLambda実行ロールを作成：

```bash
# 信頼ポリシーファイルの作成
cat > trust-policy.json << EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "lambda.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
EOF

# IAMロールの作成
aws iam create-role \
  --role-name lambda-execution-role \
  --assume-role-policy-document file://trust-policy.json

# 基本実行権限のアタッチ
aws iam attach-role-policy \
  --role-name lambda-execution-role \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole

# ロールARNの確認
aws iam get-role --role-name lambda-execution-role --query 'Role.Arn' --output text
```

### デプロイ手順

#### 方法1: デプロイスクリプトを使用（推奨）
```bash
# 実行権限の付与（初回のみ）
chmod +x deploy.sh

# デプロイ実行
./deploy.sh slack-attendance arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role
```

#### 方法2: cargo lambdaコマンドを直接使用
```bash
# ビルド
cargo lambda build --release

# デプロイ
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance
```

#### 方法3: 環境変数付きでデプロイ
```bash
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  --env-vars SLACK_SIGNING_SECRET=your_secret,NOTION_API_KEY=your_key,NOTION_DATABASE_ID=your_db_id \
  slack-attendance
```

### API Gateway設定

デプロイ後、API Gatewayを手動で設定する必要があります：

1. **Lambda関数の確認**
   ```bash
   aws lambda list-functions --query 'Functions[?FunctionName==`slack-attendance`]'
   ```

2. **API Gatewayの作成**
   - AWS ConsoleでAPI Gateway（REST API）を作成
   - リソース作成: `/slack`
   - メソッド作成: `POST`
   - Lambda統合設定で作成した関数を指定

3. **デプロイステージの作成**
   - ステージ名: `prod`
   - デプロイ実行

### Slack設定

1. **Slackアプリでエンドポイント設定**
   - Slack App管理画面
   - Slash Commands設定
   - Request URL: `https://API_ID.execute-api.REGION.amazonaws.com/prod/slack`

2. **環境変数の設定**
   ```bash
   # Lambda関数の環境変数設定
   aws lambda update-function-configuration \
     --function-name slack-attendance \
     --environment Variables='{
       "SLACK_SIGNING_SECRET":"your_slack_signing_secret",
       "NOTION_API_KEY":"your_notion_api_key", 
       "NOTION_DATABASE_ID":"your_notion_database_id"
     }'
   ```

## 必要な環境変数

| 環境変数名 | 説明 | 取得方法 |
|-----------|------|---------|
| `SLACK_SIGNING_SECRET` | Slack署名検証用 | Slack App設定 > Basic Information > Signing Secret |
| `NOTION_API_KEY` | Notion API接続用 | Notion > Settings & members > Integrations > 新しい統合を作成 |
| `NOTION_DATABASE_ID` | 勤怠データベース | NotionデータベースURLの32文字の文字列 |

## Notionデータベース設定

以下のプロパティを持つNotionデータベースを作成してください：

| プロパティ名 | タイプ | 設定 |
|-------------|-------|------|
| ユーザーID | Title | - |
| ユーザー名 | Text | - |
| アクション | Select | オプション: 出勤、休憩入り、休憩戻り、退勤 |
| タイムスタンプ | Date | 時刻を含む |
| 日付 | Text | - |

## トラブルシューティング

### よくあるエラーと解決方法

#### 1. ビルドエラー
```bash
# Zigが見つからない場合
cargo lambda build --release --target x86_64-unknown-linux-gnu

# OpenSSL関連エラー
# → rustls-tlsを使用しているため通常は発生しないはず
```

#### 2. デプロイエラー
```bash
# IAMロール権限不足
Error: AccessDenied: User: arn:aws:iam::ACCOUNT:user/USER is not authorized

# 解決: IAM権限の確認とロール作成権限の付与
aws iam attach-user-policy \
  --user-name YOUR_USER \
  --policy-arn arn:aws:iam::aws:policy/IAMFullAccess
```

#### 3. 実行時エラー
```bash
# ログの確認
aws logs describe-log-groups --log-group-name-prefix "/aws/lambda/slack-attendance"

# 直近のログストリーム確認
aws logs describe-log-streams \
  --log-group-name "/aws/lambda/slack-attendance" \
  --order-by LastEventTime \
  --descending \
  --max-items 1

# ログの表示
aws logs get-log-events \
  --log-group-name "/aws/lambda/slack-attendance" \
  --log-stream-name "LOG_STREAM_NAME"
```

#### 4. 環境変数エラー
```bash
# 環境変数の確認
aws lambda get-function-configuration \
  --function-name slack-attendance \
  --query 'Environment.Variables'
```

## 使用方法

### 基本コマンド

```
/attendance in      # 出勤
/attendance break   # 休憩開始
/attendance back    # 休憩終了
/attendance out     # 退勤
/attendance report  # 月次レポート表示
```

### レスポンス例

```
田中太郎 さんが 出勤 しました (2024-06-13 09:00:00)
```

## ライセンス

MIT License

詳細については [Cargo Lambda ドキュメント](https://www.cargo-lambda.info/commands/deploy.html) を参照してください。
