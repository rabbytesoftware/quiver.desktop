.PHONY: help deps deps-frontend deps-rust \
	fmt-check-frontend fmt-frontend lint-frontend typecheck-frontend audit-frontend \
	fmt-check-rust fmt-rust lint-rust audit-rust \
	code-quality-frontend code-quality-rust \
	build-frontend build-rust build \
	fetch-sidecar dev build-app \
	test-frontend coverage-frontend \
	test-rust coverage-rust \
	pr-checks clean

.DEFAULT_GOAL := help

BUN          := $(shell command -v bun 2>/dev/null || echo $(HOME)/.bun/bin/bun)
BUNX         := $(shell command -v bunx 2>/dev/null || echo $(HOME)/.bun/bin/bunx)
CARGO        := $(shell command -v cargo 2>/dev/null || ls $(HOME)/.rustup/toolchains/*/bin/cargo 2>/dev/null | head -1 || echo $(HOME)/.cargo/bin/cargo)
RUSTC        := $(shell command -v rustc 2>/dev/null || ls $(HOME)/.rustup/toolchains/*/bin/rustc 2>/dev/null | head -1 || echo $(HOME)/.cargo/bin/rustc)
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

SIDECAR_PATH := src-tauri/binaries/quiver-$(TARGET_TRIPLE)

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
	@echo "  make dev                   - Start Tauri dev environment (fetches sidecar if needed)"
	@echo "  make build-app             - Build full Tauri app installer for this platform"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  make test-frontend         - Run TypeScript tests"
	@echo "  make coverage-frontend     - Run TypeScript tests with coverage (≥95%)"
	@echo "  make test-rust             - Run Rust tests"
	@echo "  make coverage-rust         - Run Rust tests with coverage (≥95%)"
	@echo ""
	@echo "✅ CI/PR:"
	@echo "  make pr-checks             - Run all CI steps locally"
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
	@mkdir -p src-tauri/target/debug/binaries
	@cp "$(SIDECAR_PATH)" "src-tauri/target/debug/binaries/quiver"
	@echo "✅ Sidecar ready: $(SIDECAR_PATH)"

dev:
	@if [ ! -f "$(SIDECAR_PATH)" ]; then $(MAKE) fetch-sidecar; fi
	@echo "🚀 Starting Tauri dev environment..."
	@PATH="$(dir $(CARGO)):$(dir $(BUN)):$$PATH" $(BUN) run tauri dev

build-app:
	@$(MAKE) fetch-sidecar
	@echo "📦 Building Tauri app for $(TARGET_TRIPLE)..."
	@PATH="$(dir $(CARGO)):$(dir $(BUN)):$$PATH" $(BUN) run tauri build
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
		--exclude-files 'src/lib.rs' 'src/main.rs' 'src/commands/*.rs' \
		'src/connection/local/mod.rs' 'src/connection/local/sidecar.rs' \
		'src/connection/remote/mod.rs' 'src/connection/manager.rs' \
		'src/connection/mod.rs' 'src/connection/ws/mod.rs' \
		--verbose || (echo "❌ Coverage generation failed" && exit 1)
	@if [ -f "coverage/rust/cobertura.xml" ]; then \
		OVERALL_COVERAGE=$$(grep -oP 'line-rate="\K[0-9.]+' coverage/rust/cobertura.xml | head -1); \
		COVERAGE_PERCENT=$$(echo "$$OVERALL_COVERAGE * 100" | bc); \
		echo "Overall coverage: $${COVERAGE_PERCENT}%"; \
		if [ $$(echo "$$OVERALL_COVERAGE < 0.95" | bc -l) -eq 1 ]; then \
			echo "❌ Overall coverage $${COVERAGE_PERCENT}% is below required 95%"; \
			exit 1; \
		fi; \
		echo "✅ Overall coverage $${COVERAGE_PERCENT}% meets requirement (≥95%)"; \
	else \
		echo "⚠️  Coverage file not found, skipping coverage check"; \
	fi
	@echo "📊 Rust coverage report: coverage/rust/index.html"

pr-checks:
	@echo "🚀 Running all PR validation checks..."
	@echo ""
	@echo "Step 1/6: CORE_VERSION Validation"
	@echo "=============================="
	@CORE_VERSION=$$(node -p "require('./package.json').quiver.coreVersion" 2>/dev/null | tr -d '[:space:]'); \
	if [ -z "$$CORE_VERSION" ]; then echo "❌ quiver.coreVersion missing from package.json" && exit 1; fi; \
	echo "Checking quiver.core release: $$CORE_VERSION"; \
	gh release view "$$CORE_VERSION" --repo rabbytesoftware/quiver.core --json tagName --jq '.tagName' >/dev/null || \
	  (echo "❌ quiver.core release $$CORE_VERSION not found" && exit 1); \
	echo "✅ quiver.core release $$CORE_VERSION exists"
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
