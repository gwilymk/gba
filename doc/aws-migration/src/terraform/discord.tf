# ==============================================================================
# IAM Role
# ==============================================================================

resource "aws_iam_role" "discord_notifier" {
  name = "agb-discord-notifier-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "discord_notifier_basic" {
  role       = aws_iam_role.discord_notifier.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# ==============================================================================
# Lambda Function
# Transforms CloudWatch alarms into Discord webhook messages
# ==============================================================================

resource "aws_lambda_function" "discord_notifier" {
  function_name = "agb-discord-notifier"
  role          = aws_iam_role.discord_notifier.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]

  filename         = "${path.module}/discord_notifier/bootstrap.zip"
  source_code_hash = fileexists("${path.module}/discord_notifier/bootstrap.zip") ? filebase64sha256("${path.module}/discord_notifier/bootstrap.zip") : null

  timeout     = 10
  memory_size = 128

  environment {
    variables = { DISCORD_WEBHOOK_URL = var.discord_webhook_url }
  }

  tags = { Name = "agb-discord-notifier" }
}

resource "aws_lambda_permission" "discord_notifier_sns" {
  statement_id  = "AllowSNSInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.discord_notifier.function_name
  principal     = "sns.amazonaws.com"
  source_arn    = aws_sns_topic.playground_alerts.arn
}
