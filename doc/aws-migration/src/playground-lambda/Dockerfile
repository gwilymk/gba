# Lambda container image for the playground build service
# Runs on ARM64 (Graviton2) for 20% cost savings

FROM --platform=linux/arm64 public.ecr.aws/lambda/provided:al2023-arm64

# Install Rust toolchain (ARM64 native)
RUN dnf install -y gcc make tar gzip && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain nightly --profile minimal && \
    source ~/.cargo/env && \
    rustup target add thumbv4t-none-eabi && \
    rustup component add rust-src

# Install agb-gbafix
RUN source ~/.cargo/env && \
    cargo install agb-gbafix

# Pre-cache agb dependencies (this makes incremental builds fast)
WORKDIR /var/task/template
COPY template/ .
RUN source ~/.cargo/env && \
    cargo add agb && \
    cargo add agb_tracker && \
    cargo build --release 2>/dev/null || true

# Copy the Lambda handler (Rust binary)
COPY target/lambda/playground-lambda/bootstrap ${LAMBDA_RUNTIME_DIR}/bootstrap

# Copy pre-built target directory for faster incremental builds
RUN cp -r /var/task/template/target /var/task/cached-target

WORKDIR /var/task
CMD ["handler"]
