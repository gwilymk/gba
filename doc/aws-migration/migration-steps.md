# Migration Steps

## Prerequisites

- AWS account
- Cloudflare account with DNS access to agbrs.dev
- GitHub repository admin access (for secrets)
- Local tools: `cargo-lambda`, `terraform`, `docker`, `aws` CLI

## Step 1: AWS Credentials

You need temporary AWS credentials for the bootstrap. Options:

**Option A: Use existing credentials**
```bash
aws configure
# Enter your access key, secret, and region (us-west-2)
```

**Option B: Create temporary admin user**
```bash
aws iam create-user --user-name agb-bootstrap
aws iam attach-user-policy --user-name agb-bootstrap \
  --policy-arn arn:aws:iam::aws:policy/AdministratorAccess
aws iam create-access-key --user-name agb-bootstrap
# Use these credentials with 'aws configure'
```

## Step 2: Create Discord Webhook

1. Open Discord server settings → Integrations → Webhooks
2. Create webhook, name it "AGB Playground Alerts"
3. Copy the webhook URL

## Step 3: Create terraform.tfvars

```bash
cp doc/aws-migration/src/terraform/terraform.tfvars.example infrastructure/terraform.tfvars
```

Edit `infrastructure/terraform.tfvars`:
```hcl
cloudflare_api_token = "your-cloudflare-api-token"
cloudflare_zone_id   = "your-zone-id"
discord_webhook_url  = "https://discord.com/api/webhooks/..."
```

## Step 4: Run Bootstrap

```bash
./doc/aws-migration/src/terraform/bootstrap.sh
```

This script:
1. Creates S3 bucket for Terraform state
2. Creates ECR repository
3. Builds and pushes the playground container image
4. Builds the Discord notifier Lambda
5. Runs `terraform apply` to create everything else
6. Outputs the `AWS_ROLE_ARN` you need for GitHub

## Step 5: Add GitHub Secrets

Go to **Settings → Secrets and variables → Actions** and add:

| Secret                 | Value                                |
| ---------------------- | ------------------------------------ |
| `AWS_ROLE_ARN`         | Output from bootstrap script         |
| `CLOUDFLARE_API_TOKEN` | Your Cloudflare API token            |
| `CLOUDFLARE_ZONE_ID`   | Your Cloudflare zone ID              |
| `DISCORD_WEBHOOK_URL`  | Discord webhook URL                  |

## Step 6: Copy Workflow Files

```bash
cp doc/aws-migration/src/workflows/*.yml .github/workflows/
```

## Step 7: Test

1. Visit https://play.agbrs.dev and try a build
2. Trigger **Actions → Emergency Controls → status** to verify CI/CD works
3. Test Discord alerts:
   ```bash
   aws cloudwatch set-alarm-state \
     --alarm-name agb-playground-errors \
     --state-value ALARM \
     --state-reason "Testing"
   ```

## Step 8: Clean Up

Once everything works:

```bash
# Delete bootstrap credentials (if you created them)
aws iam delete-access-key --user-name agb-bootstrap --access-key-id AKIA...
aws iam detach-user-policy --user-name agb-bootstrap \
  --policy-arn arn:aws:iam::aws:policy/AdministratorAccess
aws iam delete-user --user-name agb-bootstrap

# Delete old Digital Ocean resources
doctl compute droplet delete playground-server
```

## Rollback

If something goes wrong:

1. Re-deploy to Digital Ocean using the old workflow
2. Update Cloudflare DNS manually
3. Debug AWS at your leisure

Keep the old deployment code until you're confident the migration is stable.
