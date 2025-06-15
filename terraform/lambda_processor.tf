# SQS Event Source Mapping for processor Lambda
resource "aws_lambda_event_source_mapping" "sqs_trigger" {
  event_source_arn = aws_sqs_queue.slack_attendance_queue.arn
  function_name    = aws_lambda_function.slack_attendance.function_name
  batch_size       = 1  # Process one message at a time for simplicity
}

# IAM policy for processor Lambda to access SQS
resource "aws_iam_role_policy" "processor_lambda_sqs_policy" {
  name = "${var.lambda_function_name}-processor-sqs-policy"
  role = aws_iam_role.lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:ReceiveMessage",
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes"
        ]
        Resource = aws_sqs_queue.slack_attendance_queue.arn
      }
    ]
  })
}

# Update the existing Lambda function name to reflect its new role
# Note: This is handled in main.tf, but we'll add the dependency here
resource "aws_lambda_function_configuration" "processor_config" {
  count = 0  # This is a placeholder to show dependencies

  depends_on = [
    aws_iam_role_policy.processor_lambda_sqs_policy
  ]
}