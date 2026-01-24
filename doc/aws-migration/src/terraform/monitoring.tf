# ==============================================================================
# SNS Topic for Alerts
# ==============================================================================

resource "aws_sns_topic" "playground_alerts" {
  name = "agb-playground-alerts"
}

resource "aws_sns_topic_subscription" "discord_notifier" {
  topic_arn = aws_sns_topic.playground_alerts.arn
  protocol  = "lambda"
  endpoint  = aws_lambda_function.discord_notifier.arn
}

resource "aws_sns_topic_subscription" "playground_alerts_email" {
  count     = var.alert_email != "" ? 1 : 0
  topic_arn = aws_sns_topic.playground_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

# ==============================================================================
# CloudWatch Alarms
# ==============================================================================

resource "aws_cloudwatch_metric_alarm" "playground_errors" {
  alarm_name          = "agb-playground-errors"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "Errors"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Sum"
  threshold           = 10
  alarm_description   = "Playground Lambda error rate too high"

  dimensions = {
    FunctionName = aws_lambda_function.playground.function_name
  }

  alarm_actions = [aws_sns_topic.playground_alerts.arn]
  ok_actions    = [aws_sns_topic.playground_alerts.arn]
}

resource "aws_cloudwatch_metric_alarm" "playground_throttles" {
  alarm_name          = "agb-playground-throttles"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "Throttles"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Sum"
  threshold           = 5
  alarm_description   = "Playground Lambda being throttled (possible abuse)"

  dimensions = {
    FunctionName = aws_lambda_function.playground.function_name
  }

  alarm_actions = [aws_sns_topic.playground_alerts.arn]
  ok_actions    = [aws_sns_topic.playground_alerts.arn]
}

resource "aws_cloudwatch_metric_alarm" "playground_duration" {
  alarm_name          = "agb-playground-duration"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "Duration"
  namespace           = "AWS/Lambda"
  period              = 300
  statistic           = "Average"
  threshold           = 60000
  alarm_description   = "Playground builds taking too long on average"

  dimensions = {
    FunctionName = aws_lambda_function.playground.function_name
  }

  alarm_actions = [aws_sns_topic.playground_alerts.arn]
  ok_actions    = [aws_sns_topic.playground_alerts.arn]
}

resource "aws_cloudwatch_metric_alarm" "playground_invocations" {
  alarm_name          = "agb-playground-high-usage"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "Invocations"
  namespace           = "AWS/Lambda"
  period              = 3600
  statistic           = "Sum"
  threshold           = 100
  alarm_description   = "Unusually high playground usage (possible abuse)"

  dimensions = {
    FunctionName = aws_lambda_function.playground.function_name
  }

  alarm_actions = [aws_sns_topic.playground_alerts.arn]
  ok_actions    = [aws_sns_topic.playground_alerts.arn]
}

# ==============================================================================
# CloudWatch Dashboard
# ==============================================================================

resource "aws_cloudwatch_dashboard" "playground" {
  dashboard_name = "agb-playground"

  dashboard_body = jsonencode({
    widgets = [
      {
        type   = "metric"
        x      = 0
        y      = 0
        width  = 12
        height = 6
        properties = {
          title   = "Invocations"
          region  = "us-west-2"
          metrics = [["AWS/Lambda", "Invocations", "FunctionName", aws_lambda_function.playground.function_name]]
          period  = 300
          stat    = "Sum"
        }
      },
      {
        type   = "metric"
        x      = 12
        y      = 0
        width  = 12
        height = 6
        properties = {
          title   = "Duration"
          region  = "us-west-2"
          metrics = [["AWS/Lambda", "Duration", "FunctionName", aws_lambda_function.playground.function_name]]
          period  = 300
          stat    = "Average"
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 6
        width  = 12
        height = 6
        properties = {
          title   = "Errors"
          region  = "us-west-2"
          metrics = [["AWS/Lambda", "Errors", "FunctionName", aws_lambda_function.playground.function_name]]
          period  = 300
          stat    = "Sum"
        }
      },
      {
        type   = "metric"
        x      = 12
        y      = 6
        width  = 12
        height = 6
        properties = {
          title   = "Throttles"
          region  = "us-west-2"
          metrics = [["AWS/Lambda", "Throttles", "FunctionName", aws_lambda_function.playground.function_name]]
          period  = 300
          stat    = "Sum"
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 12
        width  = 24
        height = 6
        properties = {
          title   = "Concurrent Executions"
          region  = "us-west-2"
          metrics = [["AWS/Lambda", "ConcurrentExecutions", "FunctionName", aws_lambda_function.playground.function_name]]
          period  = 60
          stat    = "Maximum"
        }
      }
    ]
  })
}
