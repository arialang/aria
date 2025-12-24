# syntax=docker/dockerfile:1
# This Docker container installs Aria from prebuilt binaries on Ubuntu 24.04.
# It can be used as a base image for Aria development or to run Aria in CI.
FROM ubuntu:24.04

# To build with a specific version, use:
#   docker build --build-arg ARIA_VERSION=<version> --build-arg ARIA_BUILD_TIMESTAMP=<timestamp> --build-arg ARIA_EXPECTED_SHA256=<sha256> -t aria:<version> .
# Example:
#   docker build --build-arg ARIA_VERSION=0.9.20251220 --build-arg ARIA_BUILD_TIMESTAMP=20251220123456 --build-arg ARIA_EXPECTED_SHA256=47cb8d9de3a2229f1a403e1c616679811f085e819c1743e263c16c2c2d001d50 -t aria:0.9.20251220 .
ARG ARIA_VERSION=0.9.20251222
ARG ARIA_BUILD_TIMESTAMP=20251222174650
ARG ARIA_EXPECTED_SHA256=47cb8d9de3a2229f1a403e1c616679811f085e819c1743e263c16c2c2d001d50

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tar findutils \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    url="https://github.com/arialang/aria/releases/download/v${ARIA_VERSION}/aria-${ARIA_VERSION}-x86_64-unknown-linux-gnu-${ARIA_BUILD_TIMESTAMP}.tgz"; \
    mkdir -p /usr/aria; \
    curl -fsSL "$url" -o /tmp/aria.tgz; \
    echo "$ARIA_EXPECTED_SHA256 /tmp/aria.tgz" | sha256sum -c -; \
    tar -xzf /tmp/aria.tgz -C /usr/aria; \
    rm -f /tmp/aria.tgz; \
    if [ ! -x /usr/aria/aria ]; then \
    aria_path="$(find /usr/aria -maxdepth 4 -type f -name aria -perm -111 | head -n1 || true)"; \
    if [ -n "$aria_path" ] && [ "$aria_path" != "/usr/aria/aria" ]; then \
    ln -sf "$aria_path" /usr/aria/aria; \
    fi; \
    fi; \
    if [ ! -x /usr/aria/aria ]; then \
    echo "Error: 'aria' executable not found under /usr/aria after extraction from $url" >&2; \
    exit 1; \
    fi; \
    ln -sf /usr/aria/aria /usr/local/bin/aria

CMD ["bash", "-lc", "printf 'Aria is available in your environment. Start it by running \"aria\"\\n'; exec bash -i"]
