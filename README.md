# Slack勤怠管理Lambda (cargo-lambda版)

slack-attendance-lambdaは、SlackとNotionを連携させた勤怠管理システムです。cargo-lambdaを使用してRustで実装されたAWS Lambda関数です。

## アーキテクチャ

このシステムは、Slackの3秒タイムアウト制限に対応するため、以下の2つのLambda関数で構成されています：

1. **受付Lambda** (`slack-attendance-receiver`)
   - Slackからのリクエストを即座に受信
   - 署名検証とSQSへのメッセージ送信
   - 3秒以内に「受付完了」レスポンスを返却

2. **処理Lambda** (`slack-attendance-lambda`) 
   - SQSトリガーでNotionAPIリクエストを非同期処理
   - 処理完了後、Slackの遅延レスポンス機能で結果を通知

```
Slack → API Gateway → 受付Lambda → SQS → 処理Lambda → Notion API
  ↑                      ↓                    ↓
  └─ 即座にレスポンス      │                    └─ 遅延レスポンス
                        └─ キューに保存
```

## 機能

- Slackのスラッシュコマンド（`/attendance`）で勤怠記録
- 出勤、休憩開始、休憩終了、退勤の記録
- Notionデータベースへの自動保存
- 月次レポート機能（`/attendance report`）
- Slack署名検証によるセキュリティ確保
- SQSによる非同期処理とリトライ機能

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)
- AWS CLI設定済み
- Slackアプリ作成済み
- Notionデータベース作成済み
- [tenv](https://github.com/tofuutils/tenv) (Terraformバージョン管理用)

## ビルド

このシステムは2つのLambda関数で構成されたワークスペースプロジェクトです。

```bash
# ワークスペース全体のビルド（推奨）
cargo build --release

# 個別のLambda関数のビルド
# 受付Lambda
cd src/receiver
cargo lambda build --release
cd ../..

# 処理Lambda
cd src/processor
cargo lambda build --release
cd ../..

# 一括ビルドとデプロイ（推奨）
./deploy-both.sh [IAM-ROLE-ARN]  # ビルドとデプロイを一括実行
```

詳細については [Cargo Lambda ドキュメント](https://www.cargo-lambda.info/commands/build.html) を参照してください。

## テスト

通常のRustユニットテストは `cargo test` で実行できます（ワークスペース全体）。

ローカルで統合テストを実行するには、各Lambda関数ディレクトリで`cargo lambda watch` と `cargo lambda invoke` コマンドを使用します。

### ローカルでのテスト実行

```bash
# ワークスペース全体のテスト
cargo test

# 受付Lambdaのローカル実行（HTTPサーバーとして）
cd src/receiver
cargo lambda watch
# 別ターミナルでcURLテスト可能

# 処理Lambdaのローカル実行（SQSイベント用）
cd src/processor
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

### デプロイ手順

#### 方法1: Terraformを使用（推奨）

インフラストラクチャをコードとして管理できるため、本番環境では推奨されます。IAMロールも自動的に作成されます。

```bash
# 0. Terraformバージョンの確認（tenvを使用）
# プロジェクトはTerraform 1.12.2を使用しています
tenv terraform list
tenv terraform install 1.12.2
tenv terraform use 1.12.2

# 1. 両方のLambda関数をビルド
# 受付Lambda
cd src/receiver
cargo lambda build --release
cd ../..

# 処理Lambda
cd src/processor
cargo lambda build --release
cd ../..

# 2. Terraformディレクトリに移動
cd terraform

# 3. terraform.tfvarsファイルを作成（terraform.tfvars.exampleを参考に）
cp terraform.tfvars.example terraform.tfvars
# terraform.tfvarsを編集して実際の値を設定

# 4. Terraformの初期化
terraform init

# 5. 実行計画の確認
terraform plan

# 6. デプロイ実行（IAMロール、Lambda関数、SQS、API Gatewayを一括作成）
terraform apply

# 7. 出力されたAPI Gateway URLをSlackアプリに設定
terraform output api_gateway_url
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
./deploy.sh slack-attendance-processor arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role
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

# または aws-vault を使用する場合
# 受付Lambda
cd src/receiver
aws-vault exec YOUR_PROFILE -- cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-receiver
cd ../..

# 処理Lambda  
cd src/processor
aws-vault exec YOUR_PROFILE -- cargo lambda deploy \
  --iam-role arn:aws:iam::ACCOUNT_ID:role/lambda-execution-role \
  slack-attendance-processor
cd ../..
```

### API Gateway設定

**Terraformを使用した場合**: API Gatewayは自動的に作成されます。`terraform output api_gateway_url`でURLを確認できます。

**手動デプロイの場合**: API Gatewayを手動で設定する必要があります：

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

## 環境変数の設定

**Terraformを使用する場合（推奨）**: 環境変数は`terraform.tfvars`ファイルで管理されます。

1. **terraform.tfvarsファイルの作成**
   ```bash
   cd terraform
   cp terraform.tfvars.example terraform.tfvars
   ```

2. **terraform.tfvarsファイルの編集**
   ```bash
   # 以下の値を実際の値に置き換えてください
   slack_signing_secret = "your-slack-signing-secret-here"
   notion_api_key      = "your-notion-api-key-here"
   notion_database_id  = "your-notion-database-id-here"
   ```

   **⚠️ セキュリティ注意事項**:
   - `terraform.tfvars`ファイルは機密情報を含むため、Gitにコミットしないでください
   - `.gitignore`で既に無視されていることを確認済みです

3. **Terraformデプロイ時に自動設定**
   ```bash
   terraform apply  # 環境変数が自動的にLambda関数に設定されます
   ```

**手動デプロイを使用する場合**: AWS CLIで環境変数を設定する必要があります。

```bash
# 受付Lambda関数の環境変数設定
aws lambda update-function-configuration \
  --function-name slack-attendance-receiver \
  --environment Variables='{"SLACK_SIGNING_SECRET":"your_slack_signing_secret","SQS_QUEUE_URL":"your_sqs_queue_url"}'

# 処理Lambda関数の環境変数設定
aws lambda update-function-configuration \
  --function-name slack-attendance-lambda \
  --environment Variables='{"NOTION_API_KEY":"your_notion_api_key","NOTION_DATABASE_ID":"your_notion_database_id"}'
```

## 必要な環境変数

### 受付Lambda (`slack-attendance-receiver`)
| 環境変数名 | 説明 | 取得方法 | Terraformでの設定 |
|-----------|------|---------|-------------------|
| `SLACK_SIGNING_SECRET` | Slack署名検証用 | Slack App設定 > Basic Information > Signing Secret | `terraform.tfvars`で設定 |
| `SQS_QUEUE_URL` | SQSキューURL | 自動設定 | Terraformが自動で設定 |

### 処理Lambda (`slack-attendance-lambda`)
| 環境変数名 | 説明 | 取得方法 | Terraformでの設定 |
|-----------|------|---------|-------------------|
| `NOTION_API_KEY` | Notion API接続用 | Notion > Settings & members > Integrations > 新しい統合を作成 | `terraform.tfvars`で設定 |
| `NOTION_DATABASE_ID` | 勤怠データベース | NotionデータベースURLの32文字の文字列 | `terraform.tfvars`で設定 |

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

**即座のレスポンス（受付Lambda）:**
```
コマンドを受け付けました。処理中です... ⏳
```

**遅延レスポンス（処理Lambda）:**
```
田中太郎 さんが 出勤 しました (2024-06-13 09:00:00)
```

## ライセンス

MIT License

詳細については [Cargo Lambda ドキュメント](https://www.cargo-lambda.info/commands/deploy.html) を参照してください。
