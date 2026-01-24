# Security Considerations

Security model for the AWS Lambda playground service.

## Threat Model

The playground accepts arbitrary Rust code from untrusted users and compiles it. Key threats:

| Threat              | Risk                              | Mitigation                                       |
| ------------------- | --------------------------------- | ------------------------------------------------ |
| Code exfiltration   | User code reads sensitive files   | Lambda has no secrets; template is public        |
| Network attacks     | User code makes outbound requests | VPC with no internet access                      |
| Resource exhaustion | Infinite loops, fork bombs        | 90s timeout, 2GB memory limit, 10 concurrent max |
| Container escape    | User breaks out of sandbox        | Firecracker microVM isolation                    |
| Credential theft    | User accesses AWS credentials     | Lambda role has minimal permissions              |
| DDoS                | Overwhelming the service          | 10 concurrent limit + CloudWatch alarms          |

## Isolation Layers

### Layer 1: Lambda Firecracker microVM

Each Lambda invocation runs in its own [Firecracker](https://firecracker-microvm.github.io/) microVM:

- Hardware-level isolation (separate kernel)
- No shared memory between invocations
- Clean slate on cold starts
- Warm containers may reuse `/tmp` (acceptable for this use case)

This is stronger isolation than Docker containers, which share the host kernel.

### Layer 2: VPC Network Isolation

The Lambda runs in a VPC with:

- No NAT gateway
- No internet gateway
- Empty security group (no ingress or egress rules)

```hcl
resource "aws_security_group" "playground" {
  name        = "agb-playground-sg"
  description = "Security group for playground Lambda - no inbound/outbound"
  vpc_id      = aws_vpc.playground.id

  # No ingress rules
  # No egress rules = no network access
}
```

User code **cannot**:

- Make HTTP requests
- Connect to databases
- Exfiltrate data over the network
- Download additional payloads

### Layer 3: Read-Only Filesystem

Lambda containers have a read-only filesystem except for `/tmp`:

| Path            | Access                                      |
| --------------- | ------------------------------------------- |
| `/var/task`     | Read-only (contains toolchain, cached deps) |
| `/tmp`          | Read-write (2GB, used for builds)           |
| Everything else | Read-only                                   |

User code cannot modify the Lambda image or persist changes between invocations.

### Layer 4: Timeout Enforcement

Hard 90-second timeout at the Lambda level:

```hcl
resource "aws_lambda_function" "playground" {
  timeout = 90  # Lambda will terminate after 90 seconds
}
```

This prevents:

- Infinite loops
- Crypto mining
- Long-running processes consuming resources

### Layer 5: Concurrency Limits

Reserved concurrency limits simultaneous executions:

```hcl
resource "aws_lambda_function" "playground" {
  reserved_concurrent_executions = 10
}
```

Even under attack, only 10 builds can run simultaneously.

## IAM Permissions

### Playground Lambda Role

Minimal permissions - only what's needed to run and log:

```hcl
resource "aws_iam_role_policy_attachment" "playground_lambda_basic" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy_attachment" "playground_lambda_vpc" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole"
}
```

The Lambda **cannot**:

- Access S3 buckets
- Call other AWS services
- Modify infrastructure
- Access secrets manager

### Discord Notifier Role

Even more minimal - only needs to log:

```hcl
resource "aws_iam_role_policy_attachment" "discord_notifier_basic" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}
```

## Comparison: Current vs Proposed

| Aspect              | Current (Docker-in-Docker) | Proposed (Lambda)    |
| ------------------- | -------------------------- | -------------------- |
| Isolation           | Docker container           | Firecracker microVM  |
| Kernel              | Shared with host           | Separate             |
| Network             | Docker network rules       | VPC with no internet |
| Escape risk         | Docker daemon running      | No daemon            |
| Patching            | Manual OS updates          | AWS managed          |
| Credential exposure | Docker socket, SSH keys    | Minimal IAM role     |

The Lambda approach provides **stronger isolation** with **less maintenance**.

## Monitoring for Abuse

CloudWatch alarms detect suspicious activity:

| Alarm                       | Threshold        | Indicates                 |
| --------------------------- | ---------------- | ------------------------- |
| `agb-playground-errors`     | >10 in 5 minutes | Possible exploit attempts |
| `agb-playground-throttles`  | >5 in 5 minutes  | Rate limit being hit      |
| `agb-playground-duration`   | >60s average     | Unusual workload          |
| `agb-playground-high-usage` | >100/hour        | Potential abuse           |

All alarms notify via Discord for immediate visibility.

## Secrets Management

| Secret               | Storage                     | Access                |
| -------------------- | --------------------------- | --------------------- |
| Discord webhook URL  | Lambda environment variable | Discord notifier only |
| Cloudflare API token | GitHub Actions secrets      | CI/CD only            |
| AWS credentials      | OIDC federation (no secrets!)| CI/CD only           |

**OIDC Federation**: GitHub Actions authenticates to AWS using OpenID Connect. No long-lived AWS credentials are stored anywhere - GitHub requests temporary credentials for each workflow run. This eliminates the risk of credential leakage.

No secrets are accessible to user code.

## What User Code CAN Do

Despite the restrictions, user code can still:

- Use all available CPU (2 vCPU equivalent with 2GB memory)
- Write up to 2GB to `/tmp`
- Run for up to 90 seconds
- Use the full Rust toolchain and agb library
- Compile valid GBA ROMs

This is sufficient for the playground's purpose while preventing abuse.

## Incident Response

If abuse is detected:

1. **Immediate**: Use the Emergency Controls workflow in GitHub Actions
   - Go to **Actions → Emergency Controls → Run workflow**
   - Select `disable-playground`
   - Works from GitHub mobile app - no AWS console needed

2. **Investigation**: CloudWatch Logs contain all build requests and errors

3. **Mitigation**:
   - Reduce concurrency limit
   - Add AWS WAF if needed (additional cost)

4. **Recovery**: Use Emergency Controls → `enable-playground` to restore service
