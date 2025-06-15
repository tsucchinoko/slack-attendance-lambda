# Data source for AWS account ID
data "aws_caller_identity" "current" {}

# CloudWatch Logs
resource "aws_cloudwatch_log_group" "lambda_logs" {
  name              = "/aws/lambda/${var.lambda_function_name}"
  retention_in_days = var.log_retention_days
}

# IAM role for Lambda
resource "aws_iam_role" "lambda_role" {
  name = "${var.lambda_function_name}-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

# IAM policy for Lambda basic execution
resource "aws_iam_role_policy_attachment" "lambda_basic_execution" {
  role       = aws_iam_role.lambda_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# Package Lambda function
data "archive_file" "lambda_zip" {
  type        = "zip"
  source_file = "../target/lambda/slack-attendance-lambda/bootstrap"
  output_path = "${path.module}/lambda_deployment.zip"
}

# Lambda function
resource "aws_lambda_function" "slack_attendance" {
  filename         = data.archive_file.lambda_zip.output_path
  function_name    = var.lambda_function_name
  role            = aws_iam_role.lambda_role.arn
  handler         = "bootstrap"
  source_code_hash = data.archive_file.lambda_zip.output_base64sha256
  runtime         = "provided.al2023"
  architectures   = ["x86_64"]
  memory_size     = var.lambda_memory_size
  timeout         = var.lambda_timeout

  environment {
    variables = {
      SLACK_SIGNING_SECRET = var.slack_signing_secret
      NOTION_API_KEY       = var.notion_api_key
      NOTION_DATABASE_ID   = var.notion_database_id
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.lambda_basic_execution,
    aws_cloudwatch_log_group.lambda_logs,
  ]
}

# API Gateway REST API
resource "aws_api_gateway_rest_api" "slack_api" {
  name        = "${var.lambda_function_name}-api"
  description = "API Gateway for Slack Attendance Lambda"

  endpoint_configuration {
    types = ["REGIONAL"]
  }
}

# API Gateway Resource
resource "aws_api_gateway_resource" "slack_resource" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  parent_id   = aws_api_gateway_rest_api.slack_api.root_resource_id
  path_part   = "slack"
}

# API Gateway Method
resource "aws_api_gateway_method" "slack_post" {
  rest_api_id   = aws_api_gateway_rest_api.slack_api.id
  resource_id   = aws_api_gateway_resource.slack_resource.id
  http_method   = "POST"
  authorization = "NONE"
}

# Note: API Gateway Integration and Lambda permission are now in lambda_receiver.tf
# as they point to the receiver Lambda function

# API Gateway Deployment
resource "aws_api_gateway_deployment" "slack_deployment" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  stage_name  = var.api_gateway_stage_name

  depends_on = [
    aws_api_gateway_integration.receiver_lambda_integration
  ]

  lifecycle {
    create_before_destroy = true
  }
}