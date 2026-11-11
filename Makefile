# Native Quiver Desktop (GPUI, Rust workspace). The Tauri/bun Makefile is kept as Makefile.tauri until the
# final cleanup commit removes it.
.PHONY: help dev dev-local build test play-box play-box-online gate fmt clippy fetch-sidecar fetch-sidecar-local package \
	test-e2e-box check-release clean

.DEFAULT_GOAL := help

CARGO := $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
# cargo shells out to rustc, which is only reachable through ~/.cargo/bin when the caller's shell did not
# source ~/.cargo/env (the normal case for a non-interactive make and for CI).
export PATH := $(HOME)/.cargo/bin:$(dir $(CARGO)):$(PATH)

TARGET_TRIPLE := $(shell rustc -vV 2>/dev/null | awk '/^host:/ {print $$2}')
ifneq ($(findstring windows,$(TARGET_TRIPLE)),)
  EXE_SUFFIX := .exe
else
  EXE_SUFFIX :=
endif
SIDECAR := target/sidecar/$(TARGET_TRIPLE)/quiver$(EXE_SUFFIX)
VERSION ?= 0.1.0

help:
	@echo "Quiver Desktop (native) - Makefile commands"
	@echo ""
	@echo "  make dev                - run the app against a real quiver.core (fetches the sidecar once)"
	@echo "  make dev-local          - run the app against a sibling quiver.core checkout (QUIVER_CORE_DEV_PATH)"
	@echo "  make build              - cargo build --release for the workspace binaries (no e2e bridge)"
	@echo "  make test               - cargo test --workspace"
	@echo "  make fmt                - cargo fmt --all"
	@echo "  make clippy             - cargo clippy --workspace --all-targets -D warnings"
	@echo "  make gate               - the whole PR gate: fmt, clippy (+ e2e-bridge feature), test, deny, audit, machete, check-deps, check-layout"
	@echo "  make fetch-sidecar      - download the quiver.core release binary for this platform into target/sidecar/"
	@echo "                            (core-version.local pins an exact tag; otherwise the newest stable)"
	@echo "  make package            - build a release and assemble the bundle for this OS in target/bundle/"
	@echo "                            VERSION=x.y.z sets the bundle version; macOS builds a universal dmg"
	@echo "  make check-release      - fail if target/release binaries contain the e2e bridge"
	@echo "  make test-e2e-box       - the end-to-end scenarios in the Linux desktop container (Docker)"
	@echo "                            QUIVER_CORE_DEV_PATH names the quiver.core checkout; TESTS=name filters"
	@echo "  make clean              - cargo clean + target/sidecar + target/bundle"

# The sidecar is looked up next to the app executable as `quiver`; the dev loop therefore needs the app binary
# to be called quiverdesktop (docs/decision-log.md, "installed names").
$(SIDECAR):
	@$(CARGO) xtask fetch-sidecar

fetch-sidecar:
	@$(CARGO) xtask fetch-sidecar

dev: $(SIDECAR)
	@$(CARGO) build -p quiver-app
	@if [ -f target/debug/quiverdesktop$(EXE_SUFFIX) ]; then cp "$(SIDECAR)" "target/debug/quiver$(EXE_SUFFIX)"; \
	else echo "note: target/debug/quiverdesktop is absent (the cargo bin is still called quiver), so the sidecar was not copied beside it"; fi
	@$(CARGO) run -p quiver-app

# The daemon is built from a sibling checkout instead of downloaded. A debug build looks for
# <checkout>/.quiver/self/quiver first and quiver.core self-installs a copy there on every boot, which would
# hide a freshly built Core: clear it before every run.
fetch-sidecar-local:
	@if [ -z "$(QUIVER_CORE_DEV_PATH)" ]; then echo "set QUIVER_CORE_DEV_PATH to a quiver.core checkout"; exit 1; fi
	@rm -rf .quiver/self
	@$(MAKE) -C "$(QUIVER_CORE_DEV_PATH)" build
	@mkdir -p "$(dir $(SIDECAR))"
	@cp "$(QUIVER_CORE_DEV_PATH)/bin/quiver" "$(SIDECAR)"
	@chmod +x "$(SIDECAR)"

dev-local: fetch-sidecar-local
	@$(MAKE) dev

build:
	@$(CARGO) build --release -p quiver-app

test:
	@$(CARGO) test --workspace

fmt:
	@$(CARGO) fmt --all

clippy:
	@$(CARGO) clippy --workspace --all-targets -- -D warnings

gate:
	@$(CARGO) xtask gate

check-release:
	@$(CARGO) xtask check-release-artifact $$(ls target/release/quiver target/release/quiverdesktop target/release/*.exe 2>/dev/null)

# Release builds NEVER enable feature e2e-bridge; the package task re-checks the binaries it bundles.
ifeq ($(shell uname -s),Darwin)
package:
	@rustup target add aarch64-apple-darwin x86_64-apple-darwin
	@$(CARGO) xtask fetch-sidecar --target aarch64-apple-darwin
	@$(CARGO) xtask fetch-sidecar --target x86_64-apple-darwin
	@$(CARGO) build --release -p quiver-app --target aarch64-apple-darwin
	@$(CARGO) build --release -p quiver-app --target x86_64-apple-darwin
	@$(CARGO) xtask package macos --universal --version $(VERSION)
else
package: $(SIDECAR) build
	@$(CARGO) xtask package --version $(VERSION)
endif

# docker/e2e: builds the app with --features e2e-bridge in the box and runs `cargo test -p quiver-e2e --test scenarios`.
test-e2e-box:
	@if [ -z "$(QUIVER_CORE_DEV_PATH)" ]; then echo "set QUIVER_CORE_DEV_PATH to a quiver.core checkout"; exit 1; fi
	@QUIVER_CORE_DEV_PATH="$(QUIVER_CORE_DEV_PATH)" docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-native.sh $(TESTS)

# docker/e2e as a playground: the app and a real quiver.core in the box, driven by hand through noVNC at
# http://localhost:6080/vnc.html (E2E_NOVNC_PORT changes the port). SKIP_BUILD=1 reuses the last build.
NIGHTLY_DATE ?= $(shell git -C $(QUIVER_CORE_DEV_PATH) log -1 --format=%cd --date=short 2>/dev/null)
play-box:
	@if [ -z "$(QUIVER_CORE_DEV_PATH)" ]; then echo "set QUIVER_CORE_DEV_PATH to a quiver.core checkout"; exit 1; fi
	@QUIVER_CORE_DEV_PATH="$(QUIVER_CORE_DEV_PATH)" docker compose -f docker/e2e/docker-compose.yml run --rm --service-ports -e PLAYGROUND=1 -e CORE_V1=nightly-$(NIGHTLY_DATE) -e CORE_V2=nightly-$(NIGHTLY_DATE)b -e CORE_V3=nightly-$(NIGHTLY_DATE)c -e CORE_V4=nightly-$(NIGHTLY_DATE)d -e CORE_V5=nightly-$(NIGHTLY_DATE)e -e DESK_V1=nightly-$(NIGHTLY_DATE) -e SKIP_BUILD="$(SKIP_BUILD)" e2e scenarios/run-native.sh

# Like play-box, but on the real internet: no GitHub stand-in, so any published arrow can be found and installed.
# E2E_NOVNC_BIND / E2E_NOVNC_PORT pick where noVNC is published (default 127.0.0.1:6080).
play-box-online:
	@if [ -z "$(QUIVER_CORE_DEV_PATH)" ]; then echo "set QUIVER_CORE_DEV_PATH to a quiver.core checkout"; exit 1; fi
	@QUIVER_CORE_DEV_PATH="$(QUIVER_CORE_DEV_PATH)" docker compose -f docker/e2e/docker-compose.yml run --rm --service-ports -e PLAYGROUND=online -e SKIP_BUILD="$(SKIP_BUILD)" e2e scenarios/run-native.sh

clean:
	@$(CARGO) clean
	@rm -rf target/sidecar target/bundle
