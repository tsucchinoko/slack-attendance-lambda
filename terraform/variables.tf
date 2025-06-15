variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "ap-northeast-1"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "prod"
}

variable "lambda_function_name" {
  description = "Name of the Lambda function"
  type        = string
  default     = "slack-attendance-lambda"
}

variable "lambda_memory_size" {
  description = "Amount of memory in MB for Lambda function"
  type        = number
  default     = 256
}

variable "lambda_timeout" {
  description = "Timeout in seconds for Lambda function"
  type        = number
  default     = 30
}

variable "slack_signing_secret" {
  description = "Slack signing secret for request verification"
  type        = string
  sensitive   = true
}

variable "notion_api_key" {
  description = "Notion API key"
  type        = string
  sensitive   = true
}

variable "notion_database_id" {
  description = "Notion database ID for attendance records"
  type        = string
  sensitive   = true
}

variable "api_gateway_stage_name" {
  description = "API Gateway stage name"
  type        = string
  default     = "prod"
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention in days"
  type        = number
  default     = 7
}

variable "enable_sqs_encryption" {
  description = "Enable KMS encryption for SQS queues"
  type        = bool
  default     = false
}

variable "kms_key_arn" {
  description = "ARN of KMS key for SQS encryption (required if enable_sqs_encryption is true)"
  type        = string
  default     = ""
}