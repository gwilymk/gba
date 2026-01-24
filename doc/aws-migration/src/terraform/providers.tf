terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }

  backend "s3" {
    bucket = "agb-terraform-state"
    key    = "playground/terraform.tfstate"
    region = "us-west-2"
  }
}

provider "aws" {
  region = "us-west-2" # Oregon - cheap and stable
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}
