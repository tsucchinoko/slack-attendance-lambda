# CLAUDE.md

このファイルは、Claude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供します。

## ビルドと開発コマンド

### cargo-lambdaを使った開発
```bash
# プロジェクトのビルド
cargo build

# リリースビルド
cargo build --release

# Lambda関数をローカルで実行
cargo lambda watch

# テストイベントで実行
cargo lambda invoke --data-file test-event.json

# Lambda関数をAWSにデプロイ（AWS CLI設定済みが必要）
cargo lambda build --release
cargo lambda deploy --iam-role arn:aws:iam::ACCOUNT:role/lambda-execution-role

# ローカルでのテスト実行
cargo lambda start
```

### 開発用コマンド
```bash
# 型チェックとコンパイル
cargo check

# リンターの実行
cargo clippy

# テストの実行
cargo test

# フォーマット
cargo fmt

# クリーンアップ
cargo clean
```

## アーキテクチャ概要

cargo-lambdaを使用したSlack勤怠管理のサーバーレスシステムです。

### 主な変更点（従来のAWS SAMから）
- `lambda_http`クレートを使用してHTTPリクエスト/レスポンスを簡素化
- `cargo lambda`コマンドによる統合されたビルド・デプロイワークフロー
- rustls-tlsを使用してOpenSSL依存関係を回避
- ネイティブなRust開発エクスペリエンス

### リクエストフロー
1. ユーザーがSlackで `/attendance [アクション]` を入力
2. SlackがAPI GatewayにPOSTリクエストを送信
3. Lambda関数がSlack署名を検証
4. アクションを解析してNotionに勤怠記録を作成
5. Slackチャンネルにレスポンスを返信

### コアモジュール
- **main.rs**: lambda_httpを使用したメインハンドラー
- **slack.rs**: Slack署名検証とコマンド解析
- **notion.rs**: Notion API連携（rustls-tls使用）
- **types.rs**: データ構造定義

### セキュリティ実装
- SlackリクエストはHMAC-SHA256で5分間の時間枠内で検証
- すべてのシークレットは環境変数で管理
- rustls-tlsによる安全なHTTPS通信

### 必要な環境変数
- `SLACK_SIGNING_SECRET`: Slackアプリ設定から取得
- `NOTION_API_KEY`: Notion統合トークン
- `NOTION_DATABASE_ID`: 対象のNotionデータベースID

### 依存関係の特徴
- `lambda_http`: AWS Lambda用のHTTPハンドリング
- `reqwest`: rustls-tlsバックエンドでHTTPクライアント
- `serde`: JSON/URLエンコード処理
- `chrono`: 日付時刻処理
- `hmac`/`sha2`: Slack署名検証
- `tracing`: ログ出力

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

# デプロイ（関数名はパッケージ名と同じにする）
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-lambda

# または aws-vault を使用する場合
aws-vault exec YOUR_PROFILE -- cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-lambda
```

#### 方法3: 環境変数付きでデプロイ
```bash
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  --env-vars SLACK_SIGNING_SECRET=your_secret,NOTION_API_KEY=your_key,NOTION_DATABASE_ID=your_db_id \
  slack-attendance-lambda
```

### API Gateway設定

デプロイ後、API Gatewayを手動で設定する必要があります：

1. **Lambda関数の確認**
   ```bash
   aws lambda list-functions --query 'Functions[?FunctionName==`slack-attendance-lambda`]'
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
   # Lambda関数の環境変数設定（1行で記述）
   aws lambda update-function-configuration \
     --function-name slack-attendance-lambda \
     --environment Variables='{"SLACK_SIGNING_SECRET":"your_slack_signing_secret","NOTION_API_KEY":"your_notion_api_key","NOTION_DATABASE_ID":"your_notion_database_id"}'

   # またはaws-vaultを使用する場合
   aws-vault exec YOUR_PROFILE -- aws lambda update-function-configuration \
     --function-name slack-attendance-lambda \
     --environment Variables='{"SLACK_SIGNING_SECRET":"your_slack_signing_secret","NOTION_API_KEY":"your_notion_api_key","NOTION_DATABASE_ID":"your_notion_database_id"}'
   ```

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
aws logs describe-log-groups --log-group-name-prefix "/aws/lambda/slack-attendance-lambda"

# 直近のログストリーム確認
aws logs describe-log-streams \
  --log-group-name "/aws/lambda/slack-attendance-lambda" \
  --order-by LastEventTime \
  --descending \
  --max-items 1

# ログの表示
aws logs get-log-events \
  --log-group-name "/aws/lambda/slack-attendance-lambda" \
  --log-stream-name "LOG_STREAM_NAME"
```

#### 4. 環境変数エラー
```bash
# 環境変数の確認
aws lambda get-function-configuration \
  --function-name slack-attendance-lambda \
  --query 'Environment.Variables'
```

### パフォーマンス最適化

#### コールドスタート対策
```bash
# プロビジョニング済み同時実行数の設定
aws lambda put-provisioned-concurrency-config \
  --function-name slack-attendance-lambda \
  --qualifier \$LATEST \
  --provisioned-concurrency-level 1
```

#### メモリとタイムアウトの調整
```bash
# メモリサイズの調整（128MB-10GB）
aws lambda update-function-configuration \
  --function-name slack-attendance-lambda \
  --memory-size 256

# タイムアウトの調整（最大15分）
aws lambda update-function-configuration \
  --function-name slack-attendance-lambda \
  --timeout 30
```

## セキュリティ設定

### IAMロールの最小権限設定

Lambda実行ロールに最小限の権限のみを付与：

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow", 
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream", 
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*"
    }
  ]
}
```

### 環境変数の安全な管理

#### AWS Systems Manager Parameter Storeの使用
```bash
# パラメータの保存（暗号化）
aws ssm put-parameter \
  --name "/slack-attendance/slack-signing-secret" \
  --value "your_secret" \
  --type "SecureString"

# Lambda関数でパラメータ取得のコード例は別途実装が必要
```

#### AWS Secrets Managerの使用
```bash
# シークレットの作成
aws secretsmanager create-secret \
  --name "slack-attendance/credentials" \
  --secret-string '{
    "slack_signing_secret": "your_secret",
    "notion_api_key": "your_key",
    "notion_database_id": "your_db_id"
  }'
```