# CloudWatch Logs for receiver Lambda
resource "aws_cloudwatch_log_group" "receiver_lambda_logs" {
  name              = "/aws/lambda/${var.lambda_function_name}-receiver"
  retention_in_days = var.log_retention_days
}

# IAM role for receiver Lambda
resource "aws_iam_role" "receiver_lambda_role" {
  name = "${var.lambda_function_name}-receiver-role"

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

# IAM policy for receiver Lambda basic execution
resource "aws_iam_role_policy_attachment" "receiver_lambda_basic_execution" {
  role       = aws_iam_role.receiver_lambda_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# IAM policy for SQS access
resource "aws_iam_role_policy" "receiver_lambda_sqs_policy" {
  name = "${var.lambda_function_name}-receiver-sqs-policy"
  role = aws_iam_role.receiver_lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:SendMessage",
          "sqs:GetQueueAttributes"
        ]
        Resource = aws_sqs_queue.slack_attendance_queue.arn
      }
    ]
  })
}

# Package receiver Lambda function
data "archive_file" "receiver_lambda_zip" {
  type        = "zip"
  source_file = "../target/lambda/bootstrap/bootstrap"
  output_path = "${path.module}/receiver_lambda_deployment.zip"
}

# Receiver Lambda function
resource "aws_lambda_function" "slack_attendance_receiver" {
  filename         = data.archive_file.receiver_lambda_zip.output_path
  function_name    = "${var.lambda_function_name}-receiver"
  role            = aws_iam_role.receiver_lambda_role.arn
  handler         = "bootstrap"
  source_code_hash = data.archive_file.receiver_lambda_zip.output_base64sha256
  runtime         = "provided.al2023"
  architectures   = ["x86_64"]
  memory_size     = 128  # Minimal memory for quick response
  timeout         = 3    # 3 seconds to meet Slack requirement

  environment {
    variables = {
      SLACK_SIGNING_SECRET = var.slack_signing_secret
      SQS_QUEUE_URL       = aws_sqs_queue.slack_attendance_queue.url
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.receiver_lambda_basic_execution,
    aws_iam_role_policy.receiver_lambda_sqs_policy,
    aws_cloudwatch_log_group.receiver_lambda_logs,
  ]
}

# Update API Gateway Integration to point to receiver Lambda
resource "aws_api_gateway_integration" "receiver_lambda_integration" {
  rest_api_id             = aws_api_gateway_rest_api.slack_api.id
  resource_id             = aws_api_gateway_resource.slack_resource.id
  http_method             = aws_api_gateway_method.slack_post.http_method
  integration_http_method = "POST"
  type                    = "AWS_PROXY"
  uri                     = aws_lambda_function.slack_attendance_receiver.invoke_arn
}

# Lambda permission for API Gateway to invoke receiver
resource "aws_lambda_permission" "receiver_api_gateway_invoke" {
  statement_id  = "AllowAPIGatewayInvokeReceiver"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.slack_attendance_receiver.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_api_gateway_rest_api.slack_api.execution_arn}/*/*"
}