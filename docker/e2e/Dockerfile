# E2E verification box for the native (GPUI) app: a disposable Linux desktop (Xvfb + openbox + Mesa lavapipe
# software Vulkan, with a VNC/noVNC view for humans), the Rust toolchain to build the app with its debug-only
# e2e bridge, and Go to build quiver.core versions from source. No browser driver, no node/bun: the scenarios
# are Rust tests (`cargo test -p quiver-e2e --test scenarios`) that drive the app over its bridge.
#
# System libraries are those of docker/native/Dockerfile (the dev container that proved GPUI renders here);
# keep the two in step. Base ubuntu:24.04 matches GitHub Actions' ubuntu-latest.
#
# The build context is THIS directory, not the repo root: the repo is bind-mounted at run time (see
# docker-compose.yml), and a repo-root context would send gigabytes of target/ to the daemon on every image build.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    DISPLAY=:1 \
    RESOLUTION=1280x800x24 \
    VNC_PORT=5900 \
    NOVNC_PORT=6080

# --- Desktop + capture + the tools the shell harness uses -------------------
# openbox is required, not decoration: without a window manager the capture is black. imagemagick/scrot/xdotool
# are for evidence and for tests that need a pixel-level look.
RUN apt-get update && apt-get install -y --no-install-recommends \
    xvfb openbox x11vnc x11-utils x11-xserver-utils xauth xdotool wmctrl scrot imagemagick \
    python3 python3-pil python3-websockify dbus-x11 gnome-keyring libsecret-1-dev \
    mesa-vulkan-drivers vulkan-tools libgl1-mesa-dri libegl1 \
    fonts-dejavu-core fonts-liberation \
    git curl wget unzip jq procps psmisc lsof openssl ca-certificates sudo xz-utils \
    && rm -rf /var/lib/apt/lists/*

ARG NOVNC_VERSION=1.5.0
RUN git clone --branch v${NOVNC_VERSION} --depth 1 \
    https://github.com/novnc/noVNC.git /opt/novnc \
    && ln -s /opt/novnc/vnc.html /opt/novnc/index.html

# --- GPUI build prerequisites (same list as docker/native/Dockerfile) --------
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang pkg-config file patchelf \
    libfontconfig-dev libfreetype-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
    libssl-dev libvulkan-dev libx11-dev libx11-xcb-dev libxcb-xkb-dev libxkbfile-dev libdbus-1-dev \
    libzstd-dev libasound2-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    && rm -rf /var/lib/apt/lists/*

# --- WebView of the arrow apps (wry = WebKitGTK on Linux, embedded in the main window) --------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*

# --- Toolchains --------------------------------------------------------------
ARG GO_VERSION=1.26.2
RUN GOARCH_DEB="$(dpkg --print-architecture)" \
    && curl -fsSL https://go.dev/dl/go${GO_VERSION}.linux-${GOARCH_DEB}.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

ARG RUST_VERSION=stable
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_VERSION} --profile minimal

# GPUI under software Vulkan: lavapipe is an "emulated GPU" GPUI refuses unless told otherwise; no Wayland here.
# Set in the image so every process that launches the app (a test, a scenario, a human over VNC) is treated alike.
ENV ZED_ALLOW_EMULATED_GPU=1 \
    VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
    WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 \
    WEBKIT_FORCE_SANDBOX=0 \
    GDK_BACKEND=x11 \
    LIBGL_ALWAYS_SOFTWARE=1 \
    NO_AT_BRIDGE=1

WORKDIR /workspace

COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

EXPOSE 6080

ENTRYPOINT ["/usr/local/bin/start.sh"]
