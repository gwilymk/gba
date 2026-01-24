# AWS Migration Proposal: Playground Service

Migrate the playground build service from a constantly-running Digital Ocean droplet to AWS Lambda, eliminating monthly costs while only paying for actual usage.

## Current State

| Component       | Current Setup                                 |
| --------------- | --------------------------------------------- |
| Server          | 512MB Digital Ocean droplet (~$5/month, 24/7) |
| Build isolation | Docker-in-Docker                              |
| Public access   | Cloudflare Tunnel → `play.agbrs.dev`          |
| Deployment      | Custom Rust tool, blue-green deploys          |
| Monitoring      | Manual                                        |

**Problem**: The server runs continuously but only serves a few builds per day.

## Proposed Architecture

```
┌─────────────────┐     HTTPS POST      ┌──────────────────────┐
│   GitHub Pages  │ ─────────────────▶  │  Lambda Function URL │
│   (Static Site) │                     │  (play.agbrs.dev)    │
└─────────────────┘                     └──────────┬───────────┘
                                                   │
                                                   ▼
                                        ┌──────────────────────┐
                                        │ Lambda Container     │
                                        │ (ARM64 - Graviton2)  │
                                        │  ┌────────────────┐  │
                                        │  │ Rust Toolchain │  │
                                        │  │ agb deps cache │  │
                                        │  │ agb-gbafix     │  │
                                        │  └────────────────┘  │
                                        │                      │
                                        │  /tmp (build dir)    │
                                        │   (no network)       │
                                        └──────────────────────┘
```

| Component       | Proposed Setup                                      |
| --------------- | --------------------------------------------------- |
| Compute         | AWS Lambda (ARM64/Graviton2)                        |
| Build isolation | Lambda microVM (Firecracker) + VPC with no internet |
| Public access   | Lambda Function URL (Cloudflare DNS only)           |
| Deployment      | GitHub Actions + Terraform                          |
| Monitoring      | CloudWatch dashboard + alarms → Discord             |
| Infrastructure  | Terraform (fully managed as code)                   |

## Cost Comparison

| Metric       | Current | Proposed                                   |
| ------------ | ------- | ------------------------------------------ |
| Monthly cost | ~$5     | **$0** (free tier)                         |
| Architecture | x86_64  | ARM64 (20% cheaper if exceeding free tier) |

### Free Tier Analysis

At 50 builds/day with 2GB memory and 30-second execution:

- **Monthly GB-seconds**: 1,500 builds × 60 = 90,000
- **Free tier allowance**: 400,000 GB-seconds
- **Utilization**: 22.5%

Even at 4× expected usage, we stay within free tier.

## Why Lambda Over Fargate?

| Factor        | Lambda                   | Fargate                                       |
| ------------- | ------------------------ | --------------------------------------------- |
| Cold start    | 5-15 seconds             | 30-60 seconds                                 |
| Free tier     | 400,000 GB-seconds/month | None                                          |
| Complexity    | Single function          | ECS, task definitions, VPC, service discovery |
| Scale to zero | Yes                      | No (minimum task always running)              |
| Maintenance   | None                     | OS updates, cluster management                |

## Key Design Decisions

### 1. ARM64 (Graviton2)

- 20% cheaper than x86_64
- Better price-performance for compiled Rust code
- Used for both playground and Discord notifier Lambdas

### 2. No Network Access

- Lambda runs in VPC with no NAT gateway or internet gateway
- Security group has no ingress or egress rules
- User code cannot make network requests

### 3. Rate Limiting

- Max 10 concurrent Lambda executions
- CloudWatch alarm if >100 builds/hour
- Additional requests are throttled (HTTP 429)

### 4. 90-Second Timeout

- Hard limit prevents abuse
- Builds typically complete in ~30 seconds
- Lambda auto-terminates runaway processes

### 5. Discord Notifications

- CloudWatch alarms → SNS → Lambda → Discord webhook
- Alerts for: errors, throttles, slow builds, high usage
- Real-time visibility into playground health

## Components

| Component         | Description                             | Source                                           |
| ----------------- | --------------------------------------- | ------------------------------------------------ |
| Playground Lambda | Compiles user code, returns GBA ROM     | [src/playground-lambda/](src/playground-lambda/) |
| Discord Notifier  | Transforms CloudWatch alarms to Discord | [src/discord-notifier/](src/discord-notifier/)   |
| Terraform         | Infrastructure as code                  | [src/terraform/](src/terraform/)                 |
| CI/CD Workflows   | Automated deployment                    | [src/workflows/](src/workflows/)                 |

## Documentation

- [Migration Steps](migration-steps.md) - Step-by-step guide to perform the migration
- [Security](security.md) - Security considerations and threat model

## Quick Reference

### GitHub Secrets Required

```
AWS_ROLE_ARN         # IAM role for OIDC (no long-lived credentials!)
CLOUDFLARE_API_TOKEN
CLOUDFLARE_ZONE_ID
DISCORD_WEBHOOK_URL
ALERT_EMAIL          # Optional
```

GitHub Actions uses OIDC federation to assume an IAM role with temporary credentials. No access keys to rotate or leak.

### Justfile Targets

```just
# Run playground Lambda locally for development
serve-playground-dev:
    cd website/play && cargo lambda watch

# Build playground Lambda for deployment
build-playground-lambda:
    cd website/play && cargo lambda build --release --arm64

# Build Discord notifier Lambda
build-discord-notifier:
    cd infrastructure/discord_notifier && cargo lambda build --release --arm64

# Run Discord notifier locally for testing
serve-discord-notifier:
    cd infrastructure/discord_notifier && cargo lambda watch
```

### Emergency Controls

Go to **Actions → Emergency Controls** in GitHub and select:
- `disable-playground` - Sets concurrency to 0 (instant kill switch)
- `enable-playground` - Restores concurrency to 10
- `status` - Shows current state and recent invocations

Works from GitHub mobile app - no AWS console needed.

### Testing Alarms

```bash
# Trigger a test alarm
aws cloudwatch set-alarm-state \
  --alarm-name agb-playground-errors \
  --state-value ALARM \
  --state-reason "Testing Discord notifications"

# Reset it
aws cloudwatch set-alarm-state \
  --alarm-name agb-playground-errors \
  --state-value OK \
  --state-reason "Test complete"
```

## Summary

This migration:

- **Eliminates** monthly hosting costs ($5/month → $0)
- **Improves** security (VPC network isolation, Lambda microVM)
- **Adds** observability (CloudWatch dashboard, Discord alerts)
- **Enables** infrastructure-as-code (Terraform)
- **Simplifies** maintenance (no OS updates, no Docker daemon)

The only trade-off is 5-15 second cold starts, which is acceptable since builds take ~30 seconds anyway.
