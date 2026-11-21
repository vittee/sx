# Builds sx and wraps it in the VHS image, which has ttyd, ffmpeg and a browser for recording.
FROM rust:1 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM ghcr.io/charmbracelet/vhs
RUN apt-get update \
    && apt-get install -y --no-install-recommends openssh-client \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/sx /usr/local/bin/sx

COPY demo/.keys/id_ed25519 demo/.keys/id_ed25519.pub /root/.ssh/
COPY demo/ssh_config /root/.ssh/config
RUN chmod 700 /root/.ssh && chmod 600 /root/.ssh/id_ed25519 /root/.ssh/config \
    && for h in git.example.com 10.0.4.17 staging; do \
         echo "$h $(cut -d' ' -f1,2 /root/.ssh/id_ed25519.pub)"; \
       done > /root/.ssh/known_hosts
