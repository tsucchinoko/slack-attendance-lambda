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
  value       = "${aws_api_gateway_deployment.slack_deployment.invoke_url}/slack"
}

output "api_gateway_id" {
  description = "ID of the API Gateway"
  value       = aws_api_gateway_rest_api.slack_api.id
}

output "cloudwatch_log_group" {
  description = "CloudWatch Logs group name"
  value       = aws_cloudwatch_log_group.lambda_logs.name
}