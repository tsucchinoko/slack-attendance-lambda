# Additional IAM policies for processor Lambda
# This supplements the basic execution role in main.tf

# Policy for processor Lambda to make HTTPS requests (for Slack response_url)
resource "aws_iam_role_policy" "processor_lambda_https_policy" {
  name = "${var.lambda_function_name}-processor-https-policy"
  role = aws_iam_role.lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:*:*:*"
      }
    ]
  })
}

# Additional permissions if using KMS for SQS encryption (optional)
resource "aws_iam_role_policy" "processor_lambda_kms_policy" {
  count = var.enable_sqs_encryption ? 1 : 0
  name  = "${var.lambda_function_name}-processor-kms-policy"
  role  = aws_iam_role.lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey"
        ]
        Resource = var.kms_key_arn
        Condition = {
          StringEquals = {
            "kms:ViaService" = "sqs.${var.aws_region}.amazonaws.com"
          }
        }
      }
    ]
  })
}