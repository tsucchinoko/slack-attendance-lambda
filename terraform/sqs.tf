# Dead Letter Queue for failed messages
resource "aws_sqs_queue" "slack_attendance_dlq" {
  name                      = "${var.lambda_function_name}-dlq"
  message_retention_seconds = 1209600  # 14 days
  
  tags = {
    Environment = var.environment
    Purpose     = "DeadLetterQueue"
  }
}

# Main SQS Queue for attendance requests
resource "aws_sqs_queue" "slack_attendance_queue" {
  name                       = "${var.lambda_function_name}-queue"
  visibility_timeout_seconds = 60  # Should be at least 6x the Lambda timeout
  message_retention_seconds  = 345600  # 4 days
  receive_wait_time_seconds  = 20  # Long polling
  
  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.slack_attendance_dlq.arn
    maxReceiveCount     = 3
  })
  
  tags = {
    Environment = var.environment
    Purpose     = "AttendanceRequestQueue"
  }
}

# Optional: Enable server-side encryption for SQS queues
resource "aws_sqs_queue_policy" "slack_attendance_queue_policy" {
  queue_url = aws_sqs_queue.slack_attendance_queue.id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
        Action = [
          "sqs:SendMessage",
          "sqs:GetQueueAttributes"
        ]
        Resource = aws_sqs_queue.slack_attendance_queue.arn
      }
    ]
  })
}