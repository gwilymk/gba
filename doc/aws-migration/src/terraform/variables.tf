variable "cloudflare_api_token" {
  type      = string
  sensitive = true
}

variable "cloudflare_zone_id" {
  type        = string
  description = "Cloudflare zone ID for agbrs.dev"
}

variable "alert_email" {
  type        = string
  description = "Email address for playground alerts (optional, can be empty if using Discord only)"
  default     = ""
}

variable "discord_webhook_url" {
  type        = string
  sensitive   = true
  description = "Discord webhook URL for alerts"
}

variable "github_org" {
  type        = string
  description = "GitHub organization or username"
  default     = "agbrs"
}

variable "github_repo" {
  type        = string
  description = "GitHub repository name"
  default     = "agb"
}
