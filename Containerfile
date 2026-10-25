FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

# Tauri 2 Linux prerequisites.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential curl wget file ca-certificates git pkg-config xz-utils \
        libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
        libayatana-appindicator3-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*

# Builds compas-sdtool (scripts/build-sdtool.sh), natively and for Windows.
RUN apt-get update && apt-get install -y --no-install-recommends \
        cmake python3 gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal \
        --default-toolchain stable --component clippy,rustfmt \
    && chmod -R a+rwX /opt/rustup /opt/cargo

# Mount point of the cache volume; writable for the unprivileged build user.
RUN mkdir /cache && chmod 777 /cache

WORKDIR /work
