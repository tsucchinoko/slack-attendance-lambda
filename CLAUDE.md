# CLAUDE.md

このファイルは、Claude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供します。

## ビルドと開発コマンド

### cargo-lambdaを使った開発

このシステムは2つのLambda関数で構成されたワークスペースプロジェクトです。

```bash
# ワークスペース全体のビルド（推奨）
cargo build
cargo build --release

# 個別のLambda関数のビルド
# 受付Lambda（HTTPハンドラー）
cd src/receiver
cargo lambda build
cargo lambda build --release
cd ../..

# 処理Lambda（SQSイベントハンドラー）
cd src/processor  
cargo lambda build
cargo lambda build --release
cd ../..

# 一括ビルドとデプロイ（推奨）
./deploy-both.sh arn:aws:iam::ACCOUNT:role/lambda-execution-role

# 受付Lambdaをローカルで実行（HTTPサーバーとして）
cd src/receiver
cargo lambda watch

# 処理Lambdaをローカルで実行（SQSイベント用）
cd src/processor
cargo lambda invoke --data-file test-event.json
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

cargo-lambdaを使用したSlack勤怠管理のサーバーレスシステムです。Slackの3秒タイムアウト制限に対応するため、2つのLambda関数で構成されています。

### システム構成

```
Slack → API Gateway → 受付Lambda → SQS → 処理Lambda → Notion API
  ↑                      ↓                    ↓
  └─ 即座にレスポンス      │                    └─ 遅延レスポンス
                        └─ キューに保存
```

### Lambda関数の役割

#### 1. 受付Lambda (`slack-attendance-receiver`)
- **目的**: Slackからのリクエストを即座に受信し、3秒以内にレスポンスを返す
- **処理内容**:
  - Slack署名検証（HMAC-SHA256）
  - リクエストデータをSQSキューに送信
  - 「受付完了」メッセージを即座に返却
- **タイムアウト**: 3秒
- **メモリ**: 128MB（最小限）

#### 2. 処理Lambda (`slack-attendance-lambda`)
- **目的**: SQSトリガーでNotionAPIとの通信を非同期処理
- **処理内容**:
  - SQSメッセージからSlackコマンドを取得
  - Notion APIで勤怠記録を作成
  - Slackの遅延レスポンス機能で結果を通知
- **タイムアウト**: 30秒
- **メモリ**: 256MB

### 主な変更点（従来のAWS SAMから）
- 2つのLambda関数による非同期処理アーキテクチャ
- SQSによるメッセージキューイングとリトライ機能
- `lambda_http`（受付Lambda）と`lambda_runtime`（処理Lambda）の使い分け
- `cargo lambda`コマンドによる統合されたビルド・デプロイワークフロー
- rustls-tlsを使用してOpenSSL依存関係を回避
- ネイティブなRust開発エクスペリエンス

### リクエストフロー
1. ユーザーがSlackで `/attendance [アクション]` を入力
2. SlackがAPI GatewayにPOSTリクエストを送信
3. 受付LambdaがSlack署名を検証
4. リクエストデータをSQSキューに送信
5. 「受付完了」メッセージを即座にSlackに返信
6. SQSトリガーで処理Lambdaが起動
7. アクションを解析してNotionに勤怠記録を作成
8. Slackの遅延レスポンス機能で結果を通知

### コアモジュール

#### 受付Lambda (`src/receiver/`)
- **main.rs**: lambda_httpを使用したHTTPハンドラー
- **slack.rs**: Slack署名検証
- **types.rs**: 共有データ構造（シンボリックリンク）

#### 処理Lambda (`src/`)
- **main.rs**: lambda_runtimeを使用したSQSイベントハンドラー
- **slack.rs**: Slack署名検証とコマンド解析
- **notion.rs**: Notion API連携（rustls-tls使用）
- **types.rs**: データ構造定義

### セキュリティ実装
- SlackリクエストはHMAC-SHA256で5分間の時間枠内で検証
- すべてのシークレットは環境変数で管理
- rustls-tlsによる安全なHTTPS通信
- SQSキューとデッドレターキューによる信頼性確保

### 必要な環境変数

#### 受付Lambda
- `SLACK_SIGNING_SECRET`: Slackアプリ設定から取得
- `SQS_QUEUE_URL`: SQSキューURL（Terraformで自動設定）

#### 処理Lambda  
- `NOTION_API_KEY`: Notion統合トークン
- `NOTION_DATABASE_ID`: 対象のNotionデータベースID

### 依存関係の特徴

#### 受付Lambda
- `lambda_http`: AWS Lambda用のHTTPハンドリング
- `aws-sdk-sqs`: SQSメッセージ送信
- `serde`: JSON/URLエンコード処理
- `hmac`/`sha2`: Slack署名検証
- `tracing`: ログ出力

#### 処理Lambda
- `lambda_runtime`: AWS Lambda用のランタイム（SQSイベント）
- `aws_lambda_events`: SQSイベント構造体
- `reqwest`: rustls-tlsバックエンドでHTTPクライアント
- `serde`: JSON/URLエンコード処理
- `chrono`: 日付時刻処理
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

### デプロイ手順

#### 方法1: Terraformを使用（推奨）

IAMロール、Lambda関数、SQS、API Gatewayをすべて自動作成します。

```bash
# 1. 両方のLambda関数をビルド
cd src/receiver
cargo lambda build --release
cd ../..
cd src/processor
cargo lambda build --release
cd ../..

# 2. Terraformでインフラをデプロイ（IAMロールも自動作成）
cd terraform
terraform init
terraform plan
terraform apply
```

#### 方法2: 一括デプロイスクリプトを使用（推奨）
```bash
# 実行権限の付与（初回のみ）
chmod +x deploy-both.sh

# 両方のLambda関数を一括ビルド・デプロイ
./deploy-both.sh arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role
```

#### 方法3: 個別デプロイスクリプトを使用
```bash
# 実行権限の付与（初回のみ）
chmod +x deploy.sh

# 処理Lambdaのみデプロイ
./deploy.sh slack-attendance arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role
```

#### 方法4: cargo lambdaコマンドを直接使用
```bash
# 受付Lambdaのビルドとデプロイ
cd src/receiver
cargo lambda build --release
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-receiver
cd ../..

# 処理Lambdaのビルドとデプロイ
cd src/processor
cargo lambda build --release
cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-processor
cd ../..
```

### API Gateway設定

**Terraformを使用した場合**: API Gatewayは自動的に作成されます。`terraform output api_gateway_url`でURLを確認できます。

**手動デプロイの場合**: API Gatewayを手動で設定する必要があります：

1. **Lambda関数の確認**
   ```bash
   aws lambda list-functions --query 'Functions[?FunctionName==`slack-attendance-receiver`]'
   ```

2. **API Gatewayの作成**
   - AWS ConsoleでAPI Gateway（REST API）を作成
   - リソース作成: `/slack`
   - メソッド作成: `POST`
   - Lambda統合設定で**受付Lambda**（slack-attendance-receiver）を指定

3. **デプロイステージの作成**
   - ステージ名: `prod`
   - デプロイ実行

### Slack設定

1. **Slackアプリでエンドポイント設定**
   - Slack App管理画面
   - Slash Commands設定
   - Request URL: `https://API_ID.execute-api.REGION.amazonaws.com/prod/slack`

2. **環境変数の設定**

   **Terraformを使用する場合（推奨）**: 
   ```bash
   # terraform.tfvarsファイルを編集して環境変数を設定
   cd terraform
   cp terraform.tfvars.example terraform.tfvars
   # 以下の値を実際の値に編集
   # slack_signing_secret = "your-slack-signing-secret-here"
   # notion_api_key      = "your-notion-api-key-here"
   # notion_database_id  = "your-notion-database-id-here"
   
   # Terraformデプロイ時に環境変数が自動設定されます
   terraform apply
   ```

   **手動デプロイの場合**: 
   ```bash
   # 受付Lambda関数の環境変数設定
   aws lambda update-function-configuration \
     --function-name slack-attendance-receiver \
     --environment Variables='{"SLACK_SIGNING_SECRET":"your_slack_signing_secret","SQS_QUEUE_URL":"your_sqs_queue_url"}'

   # 処理Lambda関数の環境変数設定
   aws lambda update-function-configuration \
     --function-name slack-attendance-lambda \
     --environment Variables='{"NOTION_API_KEY":"your_notion_api_key","NOTION_DATABASE_ID":"your_notion_database_id"}'

   # SQS_QUEUE_URLはterraform outputで確認可能
   terraform output sqs_queue_url
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
# 受付Lambdaのログ確認
aws logs describe-log-groups --log-group-name-prefix "/aws/lambda/slack-attendance-receiver"
aws logs get-log-events \
  --log-group-name "/aws/lambda/slack-attendance-receiver" \
  --log-stream-name "LOG_STREAM_NAME"

# 処理Lambdaのログ確認  
aws logs describe-log-groups --log-group-name-prefix "/aws/lambda/slack-attendance-lambda"
aws logs get-log-events \
  --log-group-name "/aws/lambda/slack-attendance-lambda" \
  --log-stream-name "LOG_STREAM_NAME"

# SQSデッドレターキューの確認
aws sqs get-queue-attributes \
  --queue-url "$(terraform output -raw sqs_dlq_arn | sed 's/arn:aws:sqs:[^:]*:[^:]*:/https:\/\/sqs.ap-northeast-1.amazonaws.com\//')" \
  --attribute-names ApproximateNumberOfMessages
```

#### 4. 環境変数エラー
```bash
# 受付Lambda環境変数の確認
aws lambda get-function-configuration \
  --function-name slack-attendance-receiver \
  --query 'Environment.Variables'

# 処理Lambda環境変数の確認
aws lambda get-function-configuration \
  --function-name slack-attendance-lambda \
  --query 'Environment.Variables'
```

### パフォーマンス最適化

#### コールドスタート対策
```bash
# 受付Lambda用（3秒レスポンス要求のため重要）
aws lambda put-provisioned-concurrency-config \
  --function-name slack-attendance-receiver \
  --qualifier \$LATEST \
  --provisioned-concurrency-level 1

# 処理Lambda用（必要に応じて）
aws lambda put-provisioned-concurrency-config \
  --function-name slack-attendance-lambda \
  --qualifier \$LATEST \
  --provisioned-concurrency-level 1
```

#### メモリとタイムアウトの調整
```bash
# 受付Lambda: メモリサイズの調整（128MB推奨）
aws lambda update-function-configuration \
  --function-name slack-attendance-receiver \
  --memory-size 128

# 受付Lambda: タイムアウトの調整（3秒）
aws lambda update-function-configuration \
  --function-name slack-attendance-receiver \
  --timeout 3

# 処理Lambda: メモリサイズの調整（256MB推奨）
aws lambda update-function-configuration \
  --function-name slack-attendance-lambda \
  --memory-size 256

# 処理Lambda: タイムアウトの調整（30秒）
aws lambda update-function-configuration \
  --function-name slack-attendance-lambda \
  --timeout 30
```

## セキュリティ設定

### IAMロールの最小権限設定

各Lambda関数に必要最小限の権限のみを付与：

#### 受付Lambda用ポリシー
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
    },
    {
      "Effect": "Allow",
      "Action": [
        "sqs:SendMessage",
        "sqs:GetQueueAttributes"
      ],
      "Resource": "arn:aws:sqs:REGION:ACCOUNT:slack-attendance-lambda-queue"
    }
  ]
}
```

#### 処理Lambda用ポリシー
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
    },
    {
      "Effect": "Allow",
      "Action": [
        "sqs:ReceiveMessage",
        "sqs:DeleteMessage",
        "sqs:GetQueueAttributes"
      ],
      "Resource": "arn:aws:sqs:REGION:ACCOUNT:slack-attendance-lambda-queue"
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