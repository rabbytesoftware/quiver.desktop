.PHONY: help deps deps-frontend deps-rust \
	fmt-check-frontend fmt-frontend lint-frontend typecheck-frontend audit-frontend \
	fmt-check-rust fmt-rust lint-rust audit-rust \
	code-quality-frontend code-quality-rust \
	build-frontend build-rust build \
	fetch-sidecar dev build-app dev-bundle icon \
	test-frontend coverage-frontend \
	test-rust coverage-rust \
	pr-checks clean

.DEFAULT_GOAL := help

BUN          := $(shell command -v bun 2>/dev/null || echo $(HOME)/.bun/bin/bun)
BUNX         := $(shell command -v bunx 2>/dev/null || echo $(HOME)/.bun/bin/bunx)
CARGO        := $(shell command -v cargo 2>/dev/null || ls $(HOME)/.rustup/toolchains/*/bin/cargo 2>/dev/null | head -1 || echo $(HOME)/.cargo/bin/cargo)
RUSTC        := $(shell command -v rustc 2>/dev/null || ls $(HOME)/.rustup/toolchains/*/bin/rustc 2>/dev/null | head -1 || echo $(HOME)/.cargo/bin/rustc)
# cargo shells out to rustc, and rustc is only reachable through the toolchain
# directory when the caller's shell has not sourced ~/.cargo/env — the normal
# case for a non-interactive `make`, and for CI. Exporting once here rather than
# per-recipe is what keeps `make test-rust` from dying on "could not execute
# process `rustc -vV`" while `make dev` works. bun is here for the same reason:
# the recipes call it by bare name.
#
# ~/.cargo/bin comes BEFORE $(dir $(CARGO)) on purpose. That directory holds the
# rustup shims, which dispatch to the *default* toolchain. $(dir $(CARGO)) is
# only a fallback, and a poor one to lead with: when `command -v cargo` misses,
# CARGO falls back to `ls ~/.rustup/toolchains/*/bin/cargo | head -1`, which
# picks whichever toolchain sorts first — 1.85 on this machine, old enough that
# some dependency refuses to build with it.
export PATH  := $(HOME)/.bun/bin:$(HOME)/.cargo/bin:$(dir $(CARGO)):$(PATH)
TARGET_TRIPLE := $(shell $(RUSTC) -vV 2>/dev/null | grep '^host:' | awk '{print $$2}')
_CORE_VERSION := $(shell node -p "require('./package.json').quiver.coreVersion" 2>/dev/null | tr -d '[:space:]')

# Map target triple → quiver.core release binary name
ifeq ($(TARGET_TRIPLE),aarch64-apple-darwin)
  QUIVER_BINARY := quiver-darwin-arm64
else ifeq ($(TARGET_TRIPLE),x86_64-apple-darwin)
  QUIVER_BINARY := quiver-darwin-amd64
else ifeq ($(TARGET_TRIPLE),x86_64-unknown-linux-gnu)
  QUIVER_BINARY := quiver-linux-amd64
else ifeq ($(TARGET_TRIPLE),x86_64-pc-windows-msvc)
  QUIVER_BINARY := quiver-windows-amd64.exe
else
  QUIVER_BINARY := quiver-$(TARGET_TRIPLE)
endif

# Tauri resolves an `externalBin` as `<name>-<target triple><exe suffix>`, and
# the shell plugin resolves a sidecar as `dirname(current_exe)/<name><suffix>`.
# On Windows that suffix is `.exe` in both cases, so a sidecar written without
# it is a file neither can find — and it is invisible everywhere else, because
# the suffix is empty on every other platform. `findstring` rather than the
# exact triple: `aarch64-pc-windows-msvc` and the `-gnu` triples need it too.
# (CI does not go through here; .github/actions/build-tauri names the file
# itself. This is the local `make fetch-sidecar` / `make dev` path.)
ifneq ($(findstring windows,$(TARGET_TRIPLE)),)
  EXE_SUFFIX := .exe
else
  EXE_SUFFIX :=
endif

SIDECAR_PATH := src-tauri/binaries/quiver-$(TARGET_TRIPLE)$(EXE_SUFFIX)

# Which OS this is running on, and what that costs the bundling targets.
#
# Three of them are macOS-only and were not saying so. `icon` shells out to
# Xcode's `actool`, which exists nowhere else; `dev-bundle` ends in `open`, which
# is not a command on Linux and opens a URL handler on Windows; and both `icon`
# and `build-app` interact with `bundle.macOS.files` in tauri.conf.json, which
# maps `icons/Assets.car` — a file only `make icon` ever produces. On Linux or
# Windows the bundler is asked for a resource that was never going to exist.
#
# GUARDED rather than documented in `make help`, and deliberately: a note in the
# help text is prose that nobody re-reads and nothing checks, so it rots the
# first time a target changes. CI already encodes the same three facts as
# `if: runner.os == 'macOS'` steps (.github/actions/build-tauri), and a guard
# here is the same statement in the same form — it fires, by name, at the moment
# it is wrong, on the machine it is wrong on.
#
# UNAME_S is overridable so the guard itself can be exercised from a Mac:
# `make icon UNAME_S=Linux` must fail with the message below.
UNAME_S := $(shell uname -s)

# `bundle.macOS.files` cleared for the platforms that cannot produce its
# contents, exactly as CI does when it builds the Linux and Windows bundles.
ifeq ($(UNAME_S),Darwin)
  BUNDLE_CONFIG :=
else
  BUNDLE_CONFIG := --config '{"bundle":{"macOS":{"files":{}}}}'
endif

# Used by the macOS-only targets. `$@` names the target the user actually typed,
# and MACOS_ONLY_REASON is set per target below.
define REQUIRE_MACOS
	@if [ "$(UNAME_S)" != "Darwin" ]; then \
		echo "❌ 'make $@' is macOS-only (this is $(UNAME_S))."; \
		echo "   $(MACOS_ONLY_REASON)"; \
		echo "   'make build-app' is the target that bundles on every platform."; \
		exit 1; \
	fi
endef

help:
	@echo "Quiver Desktop - Makefile Commands"
	@echo ""
	@echo "📦 Dependencies:"
	@echo "  make deps                  - Install all dependencies (frontend + rust)"
	@echo "  make deps-frontend         - Install frontend dependencies (bun)"
	@echo "  make deps-rust             - Download Rust dependencies"
	@echo ""
	@echo "🎨 Code Quality - Frontend:"
	@echo "  make fmt-check-frontend    - Check Prettier formatting"
	@echo "  make fmt-frontend          - Fix formatting with Prettier"
	@echo "  make lint-frontend         - Run ESLint"
	@echo "  make lint-frontend-fix     - Run ESLint with auto-fix"
	@echo "  make typecheck-frontend    - Run TypeScript type checking"
	@echo "  make audit-frontend        - Run security audit"
	@echo "  make code-quality-frontend - Run all frontend quality checks"
	@echo ""
	@echo "🦀 Code Quality - Rust:"
	@echo "  make fmt-check-rust        - Check Rust formatting"
	@echo "  make fmt-rust              - Fix Rust formatting"
	@echo "  make lint-rust             - Run Clippy"
	@echo "  make audit-rust            - Run cargo audit"
	@echo "  make code-quality-rust     - Run all Rust quality checks"
	@echo ""
	@echo "🔨 Build:"
	@echo "  make build-frontend        - Build frontend"
	@echo "  make build-rust            - Build Tauri backend (debug)"
	@echo "  make build                 - Build both frontend and Rust (debug)"
	@echo "  make fetch-sidecar         - Download quiver.core sidecar binary for this platform"
	@echo "  make icon                  - macOS only: compile quiver.icon → Assets.car + .icns (Xcode 26+)"
	@echo "  make dev                   - Start Tauri dev environment (fetches sidecar if needed)"
	@echo "  make dev-bundle            - macOS only: build + open a debug .app (real icon, no hot reload)"
	@echo "  make build-app             - Build full Tauri app installer for this platform"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  make test-frontend         - Run TypeScript tests"
	@echo "  make coverage-frontend     - Run TypeScript tests with coverage (≥95%)"
	@echo "  make test-rust             - Run Rust tests"
	@echo "  make coverage-rust         - Run Rust tests with coverage (≥95%)"
	@echo ""
	@echo "✅ CI/PR:"
	@echo "  make pr-checks             - Run every CI check locally: quality, build, coverage"
	@echo "                               (not the release bundling — that is 'make build-app')"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean                 - Clean build artifacts"

deps-frontend:
	@echo "📦 Installing frontend dependencies..."
	@$(BUN) install
	@echo "✅ Frontend dependencies installed"

deps-rust:
	@echo "📦 Downloading Rust dependencies..."
	@cd src-tauri && $(CARGO) fetch
	@echo "✅ Rust dependencies downloaded"

deps: deps-frontend deps-rust

fmt-check-frontend:
	@echo "🎨 Checking frontend formatting..."
	@$(BUNX) prettier --check "src/**/*.{ts,tsx,js,jsx}" || (echo "❌ Frontend formatting check failed. Run 'make fmt-frontend' to fix." && exit 1)
	@echo "✅ Frontend formatting is correct"

fmt-frontend:
	@echo "🎨 Fixing frontend formatting..."
	@$(BUNX) prettier --write "src/**/*.{ts,tsx,js,jsx}" --ignore-pattern "src/routeTree.gen.ts"
	@echo "✅ Frontend formatted successfully"

lint-frontend:
	@echo "🔍 Running ESLint..."
	@$(BUNX) eslint "src/**/*.{ts,tsx,js,jsx}" || (echo "❌ ESLint check failed" && exit 1)
	@echo "✅ ESLint passed"

lint-frontend-fix:
	@echo "🔧 Running ESLint with auto-fix..."
	@$(BUNX) eslint "src/**/*.{ts,tsx,js,jsx}" --fix
	@echo "✅ ESLint auto-fix completed"

typecheck-frontend:
	@echo "🔍 Running TypeScript type checking..."
	@$(BUNX) tsc --noEmit || (echo "❌ Type checking failed" && exit 1)
	@echo "✅ Type checking passed"

audit-frontend:
	@echo "🔒 Running frontend security audit..."
	@$(BUN) audit || (echo "⚠️  Security vulnerabilities found" && exit 1)
	@echo "✅ Security audit passed"

code-quality-frontend: fmt-check-frontend lint-frontend typecheck-frontend
	@echo "✅ All frontend code quality checks passed!"

fmt-check-rust:
	@echo "🎨 Checking Rust formatting..."
	@cd src-tauri && $(CARGO) fmt --check || (echo "❌ Rust formatting check failed. Run 'make fmt-rust' to fix." && exit 1)
	@echo "✅ Rust formatting is correct"

fmt-rust:
	@echo "🎨 Fixing Rust formatting..."
	@cd src-tauri && $(CARGO) fmt
	@echo "✅ Rust formatted successfully"

lint-rust:
	@echo "🔍 Running Clippy..."
	@cd src-tauri && $(CARGO) clippy -- -D warnings || (echo "❌ Clippy check failed" && exit 1)
	@echo "✅ Clippy passed"

audit-rust:
	@echo "🔒 Running Rust security audit..."
	@$(CARGO) audit --version >/dev/null 2>&1 || (echo "⚠️  cargo-audit not installed. Run: $(CARGO) install cargo-audit" && exit 1)
	@cd src-tauri && $(CARGO) audit || echo "⚠️  Security vulnerabilities found (non-blocking)"
	@echo "✅ Security audit passed"

code-quality-rust: fmt-check-rust lint-rust audit-rust
	@echo "✅ All Rust code quality checks passed!"

build-frontend:
	@echo "🔨 Building frontend..."
	@$(BUN) run build || (echo "❌ Frontend build failed" && exit 1)
	@if [ ! -d "dist" ]; then echo "❌ dist/ folder not found after build" && exit 1; fi
	@echo "✅ Frontend built successfully"
	@ls -lh dist/

build-rust:
	@echo "🔨 Building Tauri backend..."
	@cd src-tauri && $(CARGO) build || (echo "❌ Rust build failed" && exit 1)
	@echo "✅ Tauri backend built successfully"

build: build-frontend build-rust
	@echo "✅ All builds completed successfully!"

# The target/debug/quiver copy at the end is for `tauri dev`, which runs the
# binary straight out of target/debug — and the shell plugin resolves a sidecar
# as dirname(current_exe)/<name>, so an unbundled run looks for it right there.
# `tauri build` needs no such help: the bundler copies from src-tauri/binaries.
fetch-sidecar:
	@echo "📥 Fetching quiver.core sidecar ($(_CORE_VERSION)) for $(TARGET_TRIPLE)..."
	@if [ -z "$(_CORE_VERSION)" ]; then echo "❌ quiver.coreVersion missing from package.json" && exit 1; fi
	@mkdir -p src-tauri/binaries
	@gh release download "$(_CORE_VERSION)" \
		--repo rabbytesoftware/quiver.core \
		--pattern "$(QUIVER_BINARY)" \
		--dir src-tauri/binaries \
		--clobber
	@mv "src-tauri/binaries/$(QUIVER_BINARY)" "$(SIDECAR_PATH)"
	@chmod +x "$(SIDECAR_PATH)"
	@mkdir -p src-tauri/target/debug
	@cp "$(SIDECAR_PATH)" "src-tauri/target/debug/quiver$(EXE_SUFFIX)"
	@echo "✅ Sidecar ready: $(SIDECAR_PATH)"

dev:
	@if [ ! -f "$(SIDECAR_PATH)" ]; then $(MAKE) fetch-sidecar; fi
	@echo "🚀 Starting Tauri dev environment..."
	@$(BUN) run tauri dev

# Deliberately NOT wired into tauri.conf.json's beforeBuildCommand: that runs
# through the platform's own shell, and requiring `bash` there would make
# Windows builds depend on Git Bash being on PATH. The icon only means anything
# on macOS, so the callers that need it ask for it — here and in CI.
icon: MACOS_ONLY_REASON := It compiles src-tauri/icons/quiver.icon with Xcode's actool.
icon:
	$(REQUIRE_MACOS)
	@echo "🎨 Compiling quiver.icon → Assets.car + quiver.icns..."
	@bash icons-compiler/compile-icon.sh
	@echo "✅ Icon artifacts in src-tauri/icons/"

# `tauri dev` runs the binary straight out of target/ and never produces a
# .app, so it never shows the real icon, the Info.plist, or the bundled
# sidecar. This is the target to reach for when checking any of those. No hot
# reload — rerun it after changing source.
#
# macOS-only, and it always was: it builds a `.app` (a bundle format no other
# platform has) and ends in `open`.
dev-bundle: MACOS_ONLY_REASON := It builds a .app and opens it, and neither exists off macOS.
dev-bundle:
	$(REQUIRE_MACOS)
	@if [ ! -f "$(SIDECAR_PATH)" ]; then $(MAKE) fetch-sidecar; fi
	@$(MAKE) icon
	@echo "📦 Building debug .app bundle..."
	@$(BUN) run tauri build --debug --bundles app
	@open src-tauri/target/debug/bundle/macos/Quiver.app

# The one bundling target that works everywhere, which is why it is the one the
# guard above points at. Two things differ off macOS, and both mirror
# .github/actions/build-tauri rather than inventing a second policy:
#
#   * `make icon` is skipped — it is Xcode-only (see the target), and CI's icon
#     step is `if: runner.os == 'macOS'` for the same reason;
#   * `bundle.macOS.files` is cleared, because it maps `icons/Assets.car`, which
#     only the icon step produces. Left set, the bundler is asked for a file that
#     was never going to be there. CI passes the identical `--config` on its
#     Linux and Windows builds.
build-app:
	@$(MAKE) fetch-sidecar
	@if [ "$(UNAME_S)" = "Darwin" ]; then \
		$(MAKE) icon; \
	else \
		echo "ℹ️  $(UNAME_S): skipping the macOS icon and clearing bundle.macOS.files"; \
	fi
	@echo "📦 Building Tauri app for $(TARGET_TRIPLE)..."
	@$(BUN) run tauri build $(BUNDLE_CONFIG)
	@echo "✅ App built — check src-tauri/target/release/bundle/"

test-frontend:
	@echo "🧪 Running TypeScript tests..."
	@$(BUN) run test || (echo "❌ TypeScript tests failed" && exit 1)
	@echo "✅ TypeScript tests passed"

coverage-frontend:
	@echo "🧪 Running TypeScript tests with coverage..."
	@mkdir -p coverage/frontend
	@$(BUN) run test:coverage -- --coverage.reportsDirectory=coverage/frontend 2>&1 | tee /tmp/ts-coverage.txt; \
	if grep -q "ERROR: Coverage" /tmp/ts-coverage.txt; then \
		echo "❌ TypeScript coverage is below required 95%"; \
		exit 1; \
	fi
	@echo "✅ TypeScript coverage meets requirement (≥95%)"
	@echo "📊 TypeScript coverage report: coverage/frontend/index.html"

test-rust:
	@echo "🧪 Running Rust tests..."
	@cd src-tauri && $(CARGO) test || (echo "❌ Rust tests failed" && exit 1)
	@echo "✅ Rust tests passed"

coverage-rust:
	@echo "🧪 Running Rust tests with coverage..."
	@$(CARGO) tarpaulin --version >/dev/null 2>&1 || (echo "⚠️  cargo-tarpaulin not installed. Run: $(CARGO) install cargo-tarpaulin" && exit 1)
	@mkdir -p coverage/rust
	@cd src-tauri && $(CARGO) tarpaulin --out Xml --out Html --out Lcov --output-dir ../coverage/rust \
		--exclude-files 'src/lib.rs' 'src/main.rs' 'src/commands/*.rs' 'src/menu.rs' 'src/fdlimit/sys.rs' \
		'src/connection/local/mod.rs' 'src/connection/local/sidecar.rs' \
		'src/connection/remote/mod.rs' 'src/connection/manager.rs' \
		'src/connection/mod.rs' 'src/connection/tauri_emitter.rs' \
		--verbose || (echo "❌ Coverage generation failed" && exit 1)
	@if [ ! -f "coverage/rust/cobertura.xml" ]; then \
		echo "❌ Coverage report missing — cannot verify the threshold"; \
		exit 1; \
	fi
	@# grep -o so each attribute lands on its own line and `head -1` can take the
	# root <coverage> element's rate — the overall one. Matching with a sed
	# address instead would be wrong: tarpaulin writes the XML on a single line,
	# and a greedy `.*line-rate="..."` then captures the LAST attribute on it
	# (some package's "1"), reporting 100% for a run that was not. That is the
	# same false-pass this target already had once, via `grep -oP`.
	@OVERALL_COVERAGE=$$(grep -o 'line-rate="[0-9.]*"' coverage/rust/cobertura.xml | head -1 | sed 's/[^0-9.]//g'); \
	if [ -z "$$OVERALL_COVERAGE" ]; then \
		echo "❌ Could not parse line-rate from the coverage report"; \
		exit 1; \
	fi; \
	COVERAGE_PERCENT=$$(echo "$$OVERALL_COVERAGE * 100" | bc); \
	echo "Overall coverage: $${COVERAGE_PERCENT}%"; \
	if [ $$(echo "$$OVERALL_COVERAGE < 0.95" | bc -l) -eq 1 ]; then \
		echo "❌ Overall coverage $${COVERAGE_PERCENT}% is below required 95%"; \
		exit 1; \
	fi; \
	echo "✅ Overall coverage $${COVERAGE_PERCENT}% meets requirement (≥95%)"
	@echo "📊 Rust coverage report: coverage/rust/index.html"

pr-checks:
	@echo "🚀 Running all PR validation checks..."
	@echo ""
	@echo "Step 1/6: CORE_VERSION Validation"
	@echo "=============================="
	@CORE_VERSION=$$(node -p "require('./package.json').quiver.coreVersion" 2>/dev/null | tr -d '[:space:]'); \
	if [ -z "$$CORE_VERSION" ]; then echo "❌ quiver.coreVersion missing from package.json" && exit 1; fi; \
	case "$$CORE_VERSION" in \
	  nightly|latest|main|master|develop|HEAD) \
	    echo "❌ '$$CORE_VERSION' is a rolling tag, not an immutable release."; \
	    echo "   Pin quiver.coreVersion to a specific beta-*/stable-* tag."; \
	    exit 1 ;; \
	esac; \
	echo "Checking quiver.core release: $$CORE_VERSION"; \
	gh release view "$$CORE_VERSION" --repo rabbytesoftware/quiver.core --json tagName --jq '.tagName' >/dev/null || \
	  (echo "❌ quiver.core release $$CORE_VERSION not found" && exit 1); \
	echo "✅ quiver.core release $$CORE_VERSION exists and is immutable"
	@echo ""
	@echo "Step 2/6: Code Quality Checks"
	@echo "=============================="
	@$(MAKE) code-quality-frontend
	@$(MAKE) code-quality-rust
	@echo ""
	@echo "Step 3/6: Build Validation"
	@echo "=============================="
	@$(MAKE) build
	@echo ""
	@echo "Step 4/6: TypeScript Test Coverage (≥95%)"
	@echo "=============================="
	@$(MAKE) coverage-frontend
	@echo ""
	@echo "Step 5/6: Rust Test Coverage (≥95%)"
	@echo "=============================="
	@$(MAKE) coverage-rust
	@echo ""
	@echo "=============================="
	@echo "✅ All PR checks passed!"
	@echo "=============================="
	@echo ""
	@echo "📋 Summary:"
	@echo "  ✅ quiver.core release exists"
	@echo "  ✅ Frontend code quality checks passed"
	@echo "  ✅ Rust code quality checks passed"
	@echo "  ✅ Frontend builds successfully"
	@echo "  ✅ Tauri backend builds successfully"
	@echo "  ✅ TypeScript test coverage ≥ 95%"
	@echo "  ✅ Rust test coverage ≥ 95%"
	@echo ""
	@echo "🎉 Your PR is ready for submission!"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@rm -rf dist/
	@rm -rf src-tauri/target/
	@rm -rf coverage/
	@rm -rf node_modules/.cache/
	@echo "✅ Cleaned successfully"
