# ── Build stage ───────────────────────────────────────────────────
FROM rust:1.93-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libxkbcommon-dev \
    libwayland-dev \
    libx11-dev \
    libx11-xcb-dev \
    libxcb1-dev \
    libxrandr-dev \
    libxi-dev \
    libxcursor-dev \
    libgl1-mesa-dev \
    fontconfig \
    libfontconfig1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build -p codeforge-app --release

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libxkbcommon0 \
    libwayland-client0 \
    libx11-6 \
    libx11-xcb1 \
    libxcb1 \
    libxrandr2 \
    libxi6 \
    libxcursor1 \
    mesa-utils \
    libgl1-mesa-dri \
    libglx-mesa0 \
    fontconfig \
    fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

# Force software rendering
ENV LIBGL_ALWAYS_SOFTWARE=1
ENV GALLIUM_DRIVER=llvmpipe
ENV WGPU_BACKEND=gl
ENV ICED_BACKEND=tiny-skia
ENV MESA_GL_VERSION_OVERRIDE=3.3

# Non-root user
RUN groupadd --gid 1000 codeforge \
    && useradd --uid 1000 --gid 1000 -m codeforge

COPY --from=builder /build/target/release/codeforge-app /usr/local/bin/codeforge-app

# Persistence directory
RUN mkdir -p /home/codeforge/.codeforge && chown -R codeforge:codeforge /home/codeforge

USER codeforge
WORKDIR /home/codeforge

ENTRYPOINT ["codeforge-app"]
