output "ecr_repository_url" {
  value       = aws_ecr_repository.playground.repository_url
  description = "ECR repository URL for pushing images"
}

output "lambda_function_url" {
  value       = aws_lambda_function_url.playground.function_url
  description = "Direct Lambda function URL"
}

output "playground_url" {
  value       = "https://play.agbrs.dev"
  description = "Public playground URL via Cloudflare"
}

output "github_actions_role_arn" {
  value       = aws_iam_role.github_actions.arn
  description = "IAM role ARN for GitHub Actions to assume"
}
