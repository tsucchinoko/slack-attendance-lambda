output "lambda_function_name" {
  description = "Name of the Lambda function"
  value       = aws_lambda_function.slack_attendance.function_name
}

output "lambda_function_arn" {
  description = "ARN of the Lambda function"
  value       = aws_lambda_function.slack_attendance.arn
}

output "api_gateway_url" {
  description = "URL of the API Gateway endpoint"
  value       = "https://${aws_api_gateway_rest_api.slack_api.id}.execute-api.${var.aws_region}.amazonaws.com/${aws_api_gateway_stage.slack_stage.stage_name}/slack"
}

output "api_gateway_id" {
  description = "ID of the API Gateway"
  value       = aws_api_gateway_rest_api.slack_api.id
}

output "cloudwatch_log_group" {
  description = "CloudWatch Logs group name"
  value       = aws_cloudwatch_log_group.lambda_logs.name
}

output "sqs_queue_url" {
  description = "URL of the SQS queue"
  value       = aws_sqs_queue.slack_attendance_queue.url
}

output "sqs_queue_arn" {
  description = "ARN of the SQS queue"
  value       = aws_sqs_queue.slack_attendance_queue.arn
}

output "sqs_dlq_arn" {
  description = "ARN of the Dead Letter Queue"
  value       = aws_sqs_queue.slack_attendance_dlq.arn
}

output "receiver_lambda_function_name" {
  description = "Name of the receiver Lambda function"
  value       = aws_lambda_function.slack_attendance_receiver.function_name
}

output "receiver_lambda_function_arn" {
  description = "ARN of the receiver Lambda function"
  value       = aws_lambda_function.slack_attendance_receiver.arn
}