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
  source_file = "../target/lambda/slack-attendance-processor/bootstrap"
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
      NOTION_API_KEY     = var.notion_api_key
      NOTION_DATABASE_ID = var.notion_database_id
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

# API Gateway Request Validator
resource "aws_api_gateway_request_validator" "slack_validator" {
  name                        = "slack-request-validator"
  rest_api_id                = aws_api_gateway_rest_api.slack_api.id
  validate_request_body      = false
  validate_request_parameters = false
}

# API Gateway Method
resource "aws_api_gateway_method" "slack_post" {
  rest_api_id   = aws_api_gateway_rest_api.slack_api.id
  resource_id   = aws_api_gateway_resource.slack_resource.id
  http_method   = "POST"
  authorization = "NONE"
  
  request_validator_id = aws_api_gateway_request_validator.slack_validator.id
  
  request_parameters = {
    "method.request.header.Content-Type"              = false
    "method.request.header.X-Slack-Request-Timestamp" = false
    "method.request.header.X-Slack-Signature"         = false
  }
  
  request_models = {
    "application/x-www-form-urlencoded" = "Empty"
  }
}

# API Gateway Method Responses
resource "aws_api_gateway_method_response" "slack_response_200" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  resource_id = aws_api_gateway_resource.slack_resource.id
  http_method = aws_api_gateway_method.slack_post.http_method
  status_code = "200"

  response_parameters = {
    "method.response.header.Content-Type" = true
  }

  response_models = {
    "application/json" = "Empty"
  }
}

resource "aws_api_gateway_method_response" "slack_response_400" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  resource_id = aws_api_gateway_resource.slack_resource.id
  http_method = aws_api_gateway_method.slack_post.http_method
  status_code = "400"

  response_parameters = {
    "method.response.header.Content-Type" = true
  }

  response_models = {
    "application/json" = "Empty"
  }
}

resource "aws_api_gateway_method_response" "slack_response_500" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  resource_id = aws_api_gateway_resource.slack_resource.id
  http_method = aws_api_gateway_method.slack_post.http_method
  status_code = "500"

  response_parameters = {
    "method.response.header.Content-Type" = true
  }

  response_models = {
    "application/json" = "Empty"
  }
}

# Note: API Gateway Integration and Lambda permission are now in lambda_receiver.tf
# as they point to the receiver Lambda function

# API Gateway Deployment
resource "aws_api_gateway_deployment" "slack_deployment" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id

  depends_on = [
    aws_api_gateway_integration.receiver_lambda_integration,
    aws_api_gateway_method_response.slack_response_200,
    aws_api_gateway_method_response.slack_response_400,
    aws_api_gateway_method_response.slack_response_500,
    aws_api_gateway_integration_response.slack_integration_response_200,
    aws_api_gateway_integration_response.slack_integration_response_400,
    aws_api_gateway_integration_response.slack_integration_response_500
  ]

  # Force redeployment when configuration changes
  triggers = {
    redeployment = sha1(jsonencode([
      aws_api_gateway_method.slack_post.id,
      aws_api_gateway_integration.receiver_lambda_integration.id,
      aws_api_gateway_method_response.slack_response_200.id,
      aws_api_gateway_method_response.slack_response_400.id,
      aws_api_gateway_method_response.slack_response_500.id,
    ]))
  }

  lifecycle {
    create_before_destroy = true
  }
}

# CloudWatch Logs IAM role for API Gateway
resource "aws_iam_role" "api_gateway_cloudwatch_role" {
  name = "${var.lambda_function_name}-api-gateway-cloudwatch-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "apigateway.amazonaws.com"
        }
      }
    ]
  })
}

# IAM policy attachment for API Gateway CloudWatch logs
resource "aws_iam_role_policy_attachment" "api_gateway_cloudwatch_logs" {
  role       = aws_iam_role.api_gateway_cloudwatch_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonAPIGatewayPushToCloudWatchLogs"
}

# CloudWatch Logs group for API Gateway
resource "aws_cloudwatch_log_group" "api_gateway_logs" {
  name              = "/aws/apigateway/${var.lambda_function_name}-api"
  retention_in_days = var.log_retention_days
}

# API Gateway account settings for CloudWatch logging
resource "aws_api_gateway_account" "api_gateway_account" {
  cloudwatch_role_arn = aws_iam_role.api_gateway_cloudwatch_role.arn
}

# API Gateway Stage
resource "aws_api_gateway_stage" "slack_stage" {
  deployment_id = aws_api_gateway_deployment.slack_deployment.id
  rest_api_id   = aws_api_gateway_rest_api.slack_api.id
  stage_name    = var.api_gateway_stage_name

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_gateway_logs.arn
    format = jsonencode({
      requestId      = "$context.requestId"
      ip             = "$context.identity.sourceIp"
      caller         = "$context.identity.caller"
      user           = "$context.identity.user"
      requestTime    = "$context.requestTime"
      httpMethod     = "$context.httpMethod"
      resourcePath   = "$context.resourcePath"
      status         = "$context.status"
      protocol       = "$context.protocol"
      responseLength = "$context.responseLength"
      requestTime    = "$context.requestTimeEpoch"
      integrationLatency = "$context.integrationLatency"
      responseLatency = "$context.responseLatency"
      errorMessage   = "$context.error.message"
      errorMessageString = "$context.error.messageString"
      integrationStatus = "$context.integrationStatus"
      integrationErrorMessage = "$context.integrationErrorMessage"
    })
  }

  # Enable execution logging
  variables = {
    "loglevel" = "INFO"
    "dataTrace" = "true"
  }

  xray_tracing_enabled = true

  depends_on = [aws_api_gateway_account.api_gateway_account]
}

# API Gateway Method Settings for detailed logging
resource "aws_api_gateway_method_settings" "slack_method_settings" {
  rest_api_id = aws_api_gateway_rest_api.slack_api.id
  stage_name  = aws_api_gateway_stage.slack_stage.stage_name
  method_path = "*/*"

  settings {
    logging_level          = "INFO"
    data_trace_enabled     = true
    metrics_enabled        = true
    throttling_burst_limit = 100
    throttling_rate_limit  = 50
  }
}