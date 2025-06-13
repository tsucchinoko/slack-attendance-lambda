#!/bin/bash

# デプロイスクリプト for cargo-lambda
# 使用方法: ./deploy.sh [関数名] [IAMロールARN]

FUNCTION_NAME=${1:-"slack-attendance-lambda"}
ROLE_ARN=${2:-""}

if [ -z "$ROLE_ARN" ]; then
    echo "Error: IAM role ARN is required"
    echo "Usage: $0 [function-name] [iam-role-arn]"
    echo "Example: $0 slack-attendance arn:aws:iam::123456789012:role/lambda-execution-role"
    exit 1
fi

echo "Building Lambda function..."
cargo lambda build --release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Deploying Lambda function: $FUNCTION_NAME"
cargo lambda deploy \
    --iam-role "$ROLE_ARN" \
    "$FUNCTION_NAME"

if [ $? -eq 0 ]; then
    echo "Deployment successful!"
    echo "Function name: $FUNCTION_NAME"
    echo "You can test the function with:"
    echo "cargo lambda invoke $FUNCTION_NAME --data-file test-event.json"
else
    echo "Deployment failed!"
    exit 1
fi