FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends build-essential cmake clang pkg-config curl ca-certificates git libfontconfig-dev libfreetype-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libssl-dev libvulkan-dev libx11-dev libx11-xcb-dev libxcb-xkb-dev libxkbfile-dev libdbus-1-dev libzstd-dev libasound2-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev xvfb x11-utils scrot xdotool imagemagick mesa-vulkan-drivers vulkan-tools libgl1-mesa-dri libegl1 openbox fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
RUN rustup component add clippy rustfmt \
 && cargo install cargo-deny cargo-machete cargo-audit --locked
RUN apt-get update && apt-get install -y --no-install-recommends gnome-keyring libsecret-1-dev jq python3 python3-pil procps psmisc lsof golang-go xz-utils && rm -rf /var/lib/apt/lists/*
