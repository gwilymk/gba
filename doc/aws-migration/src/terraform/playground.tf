# ==============================================================================
# VPC for Network Isolation (no internet access)
# ==============================================================================

resource "aws_vpc" "playground" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = { Name = "agb-playground-vpc" }
}

resource "aws_subnet" "playground" {
  vpc_id            = aws_vpc.playground.id
  cidr_block        = "10.0.1.0/24"
  availability_zone = "us-west-2a"

  tags = { Name = "agb-playground-subnet" }
}

# No NAT gateway + no internet gateway = no network access
resource "aws_security_group" "playground" {
  name        = "agb-playground-sg"
  description = "Security group for playground Lambda - no inbound/outbound"
  vpc_id      = aws_vpc.playground.id

  tags = { Name = "agb-playground-sg" }
}

# ==============================================================================
# IAM Role
# ==============================================================================

resource "aws_iam_role" "playground_lambda" {
  name = "agb-playground-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "playground_lambda_basic" {
  role       = aws_iam_role.playground_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy_attachment" "playground_lambda_vpc" {
  role       = aws_iam_role.playground_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole"
}

# ==============================================================================
# ECR Repository
# ==============================================================================

resource "aws_ecr_repository" "playground" {
  name                 = "agb-playground"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_ecr_lifecycle_policy" "playground" {
  repository = aws_ecr_repository.playground.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep only last 5 images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = 5
      }
      action = { type = "expire" }
    }]
  })
}

# ==============================================================================
# Lambda Function
# Compiles user-submitted Rust code and returns a GBA ROM
# ==============================================================================

resource "aws_lambda_function" "playground" {
  function_name = "agb-playground"
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.playground.repository_url}:latest"
  role          = aws_iam_role.playground_lambda.arn
  architectures = ["arm64"]

  memory_size = 2048
  timeout     = 90

  reserved_concurrent_executions = 10

  ephemeral_storage {
    size = 2048
  }

  vpc_config {
    subnet_ids         = [aws_subnet.playground.id]
    security_group_ids = [aws_security_group.playground.id]
  }

  environment {
    variables = { RUST_BACKTRACE = "1" }
  }

  tags = { Name = "agb-playground" }
}

resource "aws_lambda_function_url" "playground" {
  function_name      = aws_lambda_function.playground.function_name
  authorization_type = "NONE"

  cors {
    allow_origins = ["https://agbrs.dev", "https://www.agbrs.dev"]
    allow_methods = ["POST", "OPTIONS"]
    allow_headers = ["content-type"]
    max_age       = 86400
  }
}

# ==============================================================================
# DNS
# ==============================================================================

resource "cloudflare_record" "playground" {
  zone_id = var.cloudflare_zone_id
  name    = "play"
  type    = "CNAME"
  content = trimsuffix(replace(aws_lambda_function_url.playground.function_url, "https://", ""), "/")
  proxied = false
  ttl     = 300
}
