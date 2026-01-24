#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Bootstrap script for AGB Playground AWS infrastructure
# Run this once to set up the initial resources, then CI/CD takes over
# =============================================================================

REGION="us-west-2"
STATE_BUCKET="agb-terraform-state"
ECR_REPO="agb-playground"

echo "=== AGB Playground Bootstrap ==="
echo ""

# =============================================================================
# Pre-flight checks - validate everything before making any changes
# =============================================================================

echo "🔍 Running pre-flight checks..."
echo ""

ERRORS=0

# Check we're in the repo root
if [[ ! -f "website/play/docker/Lambda.dockerfile" ]]; then
    echo "❌ Must run from repository root (can't find website/play/docker/Lambda.dockerfile)"
    ERRORS=$((ERRORS + 1))
fi

if [[ ! -d "doc/aws-migration/src/terraform" ]]; then
    echo "❌ Can't find doc/aws-migration/src/terraform"
    ERRORS=$((ERRORS + 1))
fi

if [[ ! -d "doc/aws-migration/src/discord-notifier" ]]; then
    echo "❌ Can't find doc/aws-migration/src/discord-notifier"
    ERRORS=$((ERRORS + 1))
fi

# Check required tools
for cmd in aws terraform docker cargo-lambda; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "❌ Missing required tool: $cmd"
        ERRORS=$((ERRORS + 1))
    else
        echo "✓ Found $cmd"
    fi
done

# Check docker is running
if ! docker info &>/dev/null; then
    echo "❌ Docker is not running"
    ERRORS=$((ERRORS + 1))
else
    echo "✓ Docker is running"
fi

# Check docker buildx is available
if ! docker buildx version &>/dev/null; then
    echo "❌ Docker buildx not available (needed for ARM64 builds)"
    ERRORS=$((ERRORS + 1))
else
    echo "✓ Docker buildx available"
fi

# Check AWS credentials
if ! aws sts get-caller-identity &>/dev/null; then
    echo "❌ AWS credentials not configured (run 'aws configure')"
    ERRORS=$((ERRORS + 1))
else
    ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    echo "✓ AWS credentials configured (account: $ACCOUNT_ID)"
fi

# Check terraform.tfvars exists
mkdir -p infrastructure
if [[ ! -f "infrastructure/terraform.tfvars" ]]; then
    echo "❌ Missing infrastructure/terraform.tfvars"
    echo ""
    echo "   Create it with:"
    echo "   cp doc/aws-migration/src/terraform/terraform.tfvars.example infrastructure/terraform.tfvars"
    echo ""
    echo "   Then edit it with your values:"
    echo "   - cloudflare_api_token"
    echo "   - cloudflare_zone_id"
    echo "   - discord_webhook_url"
    ERRORS=$((ERRORS + 1))
else
    echo "✓ Found infrastructure/terraform.tfvars"

    # Validate terraform.tfvars has required variables
    for var in cloudflare_api_token cloudflare_zone_id discord_webhook_url; do
        if ! grep -q "^${var}" infrastructure/terraform.tfvars; then
            echo "❌ Missing variable '$var' in terraform.tfvars"
            ERRORS=$((ERRORS + 1))
        fi
    done
fi

echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo "💥 Pre-flight checks failed with $ERRORS error(s). Fix the issues above and try again."
    exit 1
fi

echo "✅ All pre-flight checks passed!"
echo ""

# =============================================================================
# Confirmation
# =============================================================================

echo "This script will:"
echo "  1. Create S3 bucket: $STATE_BUCKET"
echo "  2. Create ECR repository: $ECR_REPO"
echo "  3. Build and push playground container image"
echo "  4. Build Discord notifier Lambda"
echo "  5. Copy Terraform files to infrastructure/"
echo "  6. Run terraform apply"
echo ""
echo "Region: $REGION"
echo "AWS Account: $ACCOUNT_ID"
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo ""

# =============================================================================
# Step 1: Create S3 bucket for Terraform state
# =============================================================================

echo "📦 Step 1/6: Creating Terraform state bucket..."
if aws s3api head-bucket --bucket "$STATE_BUCKET" 2>/dev/null; then
    echo "   Bucket already exists, skipping"
else
    aws s3 mb "s3://$STATE_BUCKET" --region "$REGION"
    aws s3api put-bucket-versioning \
        --bucket "$STATE_BUCKET" \
        --versioning-configuration Status=Enabled
    echo "   Created: s3://$STATE_BUCKET"
fi
echo ""

# =============================================================================
# Step 2: Create ECR repository
# =============================================================================

echo "📦 Step 2/6: Creating ECR repository..."
if aws ecr describe-repositories --repository-names "$ECR_REPO" --region "$REGION" &>/dev/null; then
    echo "   Repository already exists, skipping"
else
    aws ecr create-repository \
        --repository-name "$ECR_REPO" \
        --region "$REGION" \
        --image-scanning-configuration scanOnPush=true
    echo "   Created: $ECR_REPO"
fi

ECR_URI="$ACCOUNT_ID.dkr.ecr.$REGION.amazonaws.com/$ECR_REPO"
echo ""

# =============================================================================
# Step 3: Build and push playground image
# =============================================================================

echo "🔨 Step 3/6: Building playground Lambda..."
echo "   This may take a while (compiling Rust code)..."

(cd website/play && cargo lambda build --release --arm64)

echo "🐳 Pushing container image to ECR..."
aws ecr get-login-password --region "$REGION" | \
    docker login --username AWS --password-stdin "$ACCOUNT_ID.dkr.ecr.$REGION.amazonaws.com"

docker buildx build --platform linux/arm64 \
    -f website/play/docker/Lambda.dockerfile \
    -t "$ECR_URI:latest" \
    -t "$ECR_URI:bootstrap" \
    --push .

echo ""

# =============================================================================
# Step 4: Build Discord notifier
# =============================================================================

echo "🔨 Step 4/6: Building Discord notifier Lambda..."

mkdir -p infrastructure/discord_notifier/src
cp doc/aws-migration/src/discord-notifier/Cargo.toml infrastructure/discord_notifier/
cp doc/aws-migration/src/discord-notifier/main.rs infrastructure/discord_notifier/src/

(cd infrastructure/discord_notifier && cargo lambda build --release --arm64)

(cd infrastructure/discord_notifier/target/lambda/discord-notifier && \
    zip -j ../../../bootstrap.zip bootstrap)

echo ""

# =============================================================================
# Step 5: Copy Terraform files
# =============================================================================

echo "📋 Step 5/6: Copying Terraform files..."

for f in doc/aws-migration/src/terraform/*.tf; do
    cp "$f" infrastructure/
    echo "   Copied $(basename "$f")"
done

echo ""

# =============================================================================
# Step 6: Run Terraform
# =============================================================================

echo "🏗️  Step 6/6: Running Terraform..."

cd infrastructure
terraform init
terraform apply

ROLE_ARN=$(terraform output -raw github_actions_role_arn)

echo ""
echo "=============================================="
echo "✅ Bootstrap complete!"
echo "=============================================="
echo ""
echo "Add these secrets to GitHub (Settings → Secrets → Actions):"
echo ""
echo "   AWS_ROLE_ARN         = $ROLE_ARN"
echo "   CLOUDFLARE_API_TOKEN = (from your terraform.tfvars)"
echo "   CLOUDFLARE_ZONE_ID   = (from your terraform.tfvars)"
echo "   DISCORD_WEBHOOK_URL  = (from your terraform.tfvars)"
echo ""
echo "Then copy the workflow files:"
echo "   cp doc/aws-migration/src/workflows/*.yml .github/workflows/"
echo ""
echo "Finally, delete your bootstrap AWS credentials if you created them."
