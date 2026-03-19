# Build on Ubuntu 20.04 (glibc 2.31) for maximum Linux compatibility.
# Binaries built here run on Ubuntu 20.04+ and most modern distros.
#
# Usage:
#   docker build -t sgleam-desktop-builder desktop/
#   docker run --rm -v $(pwd):/src sgleam-desktop-builder

FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH="/opt/cargo/bin:${PATH}"

# System dependencies (minimal: just what's needed to compile)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    ca-certificates \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Rust stable
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal

WORKDIR /src

CMD ["cargo", "build", "-p", "sgleam-desktop", "--release"]
