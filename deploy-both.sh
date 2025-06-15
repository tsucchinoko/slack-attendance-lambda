#!/bin/bash

# デプロイスクリプト for both Lambda functions
# 使用方法: ./deploy-both.sh [IAMロールARN]

ROLE_ARN=${1:-""}

if [ -z "$ROLE_ARN" ]; then
    echo "Error: IAM role ARN is required"
    echo "Usage: $0 [iam-role-arn]"
    echo "Example: $0 arn:aws:iam::123456789012:role/lambda-execution-role"
    exit 1
fi

echo "===== Building Receiver Lambda function ====="
cd src/receiver
cargo lambda build --release
if [ $? -ne 0 ]; then
    echo "Receiver build failed!"
    exit 1
fi
cd ../..

echo "===== Building Processor Lambda function ====="
cargo lambda build --release
if [ $? -ne 0 ]; then
    echo "Processor build failed!"
    exit 1
fi

echo "===== Deploying Receiver Lambda function ====="
cd src/receiver
cargo lambda deploy \
    --iam-role "$ROLE_ARN" \
    "slack-attendance-receiver"
if [ $? -ne 0 ]; then
    echo "Receiver deployment failed!"
    exit 1
fi
cd ../..

echo "===== Deploying Processor Lambda function ====="
cargo lambda deploy \
    --iam-role "$ROLE_ARN" \
    "slack-attendance-lambda"
if [ $? -ne 0 ]; then
    echo "Processor deployment failed!"
    exit 1
fi

echo "===== Deployment successful! ====="
echo "Receiver function: slack-attendance-receiver"
echo "Processor function: slack-attendance-lambda"
echo ""
echo "Next steps:"
echo "1. Run terraform apply to create/update infrastructure"
echo "2. Update environment variables for both Lambda functions"