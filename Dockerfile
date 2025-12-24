# syntax=docker/dockerfile:1
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tar findutils \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    url="https://github.com/arialang/aria/releases/download/v0.9.20251222/aria-0.9.20251222-x86_64-unknown-linux-gnu-20251222174650.tgz"; \
    mkdir -p /usr/aria; \
    curl -fsSL "$url" -o /tmp/aria.tgz; \
    tar -xzf /tmp/aria.tgz -C /usr/aria; \
    rm -f /tmp/aria.tgz; \
    if [ ! -x /usr/aria/aria ]; then \
    aria_path="$(find /usr/aria -maxdepth 4 -type f -name aria -perm -111 | head -n1 || true)"; \
    if [ -n "$aria_path" ] && [ "$aria_path" != "/usr/aria/aria" ]; then \
    ln -sf "$aria_path" /usr/aria/aria; \
    fi; \
    fi; \
    test -x /usr/aria/aria; \
    ln -sf /usr/aria/aria /usr/local/bin/aria

CMD ["bash", "-lc", "echo 'Aria is available in your environment. Start it by running \"aria\"\n'; exec bash -i"]
