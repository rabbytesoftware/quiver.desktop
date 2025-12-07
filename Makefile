.PHONY: help deps deps-frontend deps-rust \
	fmt-check-frontend fmt-frontend lint-frontend typecheck-frontend audit-frontend \
	fmt-check-rust fmt-rust lint-rust audit-rust \
	code-quality-frontend code-quality-rust \
	build-frontend build-rust build \
	test-rust coverage-rust \
	pr-checks clean

.DEFAULT_GOAL := help

CARGO := $(shell command -v cargo 2> /dev/null || echo ~/.cargo/bin/cargo)

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
	@echo "  make build-rust            - Build Tauri backend"
	@echo "  make build                 - Build both frontend and Rust"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  make test-rust             - Run Rust tests"
	@echo "  make coverage-rust         - Run tests with coverage (≥80%)"
	@echo ""
	@echo "✅ CI/PR:"
	@echo "  make pr-checks             - Run all CI steps locally"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean                 - Clean build artifacts"

deps-frontend:
	@echo "📦 Installing frontend dependencies..."
	@bun install
	@echo "✅ Frontend dependencies installed"

deps-rust:
	@echo "📦 Downloading Rust dependencies..."
	@cd src-tauri && $(CARGO) fetch
	@echo "✅ Rust dependencies downloaded"

deps: deps-frontend deps-rust

fmt-check-frontend:
	@echo "🎨 Checking frontend formatting..."
	@bunx prettier --check "src/**/*.{ts,tsx,js,jsx}" || (echo "❌ Frontend formatting check failed. Run 'make fmt-frontend' to fix." && exit 1)
	@echo "✅ Frontend formatting is correct"

fmt-frontend:
	@echo "🎨 Fixing frontend formatting..."
	@bunx prettier --write "src/**/*.{ts,tsx,js,jsx}"
	@echo "✅ Frontend formatted successfully"

lint-frontend:
	@echo "🔍 Running ESLint..."
	@bunx eslint "src/**/*.{ts,tsx,js,jsx}" || (echo "❌ ESLint check failed" && exit 1)
	@echo "✅ ESLint passed"

lint-frontend-fix:
	@echo "🔧 Running ESLint with auto-fix..."
	@bunx eslint "src/**/*.{ts,tsx,js,jsx}" --fix
	@echo "✅ ESLint auto-fix completed"

typecheck-frontend:
	@echo "🔍 Running TypeScript type checking..."
	@bunx tsc --noEmit || (echo "❌ Type checking failed" && exit 1)
	@echo "✅ Type checking passed"

audit-frontend:
	@echo "🔒 Running frontend security audit..."
	@bun audit || (echo "⚠️  Security vulnerabilities found" && exit 1)
	@echo "✅ Security audit passed"

code-quality-frontend: fmt-check-frontend lint-frontend typecheck-frontend audit-frontend
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
	@cd src-tauri && $(CARGO) audit || (echo "⚠️  Security vulnerabilities found" && exit 1)
	@echo "✅ Security audit passed"

code-quality-rust: fmt-check-rust lint-rust audit-rust
	@echo "✅ All Rust code quality checks passed!"

build-frontend:
	@echo "🔨 Building frontend..."
	@bun run build || (echo "❌ Frontend build failed" && exit 1)
	@if [ ! -d "dist" ]; then echo "❌ dist/ folder not found after build" && exit 1; fi
	@echo "✅ Frontend built successfully"
	@ls -lh dist/

build-rust:
	@echo "🔨 Building Tauri backend..."
	@cd src-tauri && $(CARGO) build || (echo "❌ Rust build failed" && exit 1)
	@echo "✅ Tauri backend built successfully"

build: build-frontend build-rust
	@echo "✅ All builds completed successfully!"

test-rust:
	@echo "🧪 Running Rust tests..."
	@cd src-tauri && $(CARGO) test || (echo "❌ Rust tests failed" && exit 1)
	@echo "✅ Rust tests passed"

coverage-rust:
	@echo "🧪 Running Rust tests with coverage..."
	@$(CARGO) tarpaulin --version >/dev/null 2>&1 || (echo "⚠️  cargo-tarpaulin not installed. Run: $(CARGO) install cargo-tarpaulin" && exit 1)
	@mkdir -p coverage
	@cd src-tauri && $(CARGO) tarpaulin --out Xml --out Html --out Lcov --output-dir ../coverage --verbose || (echo "❌ Coverage generation failed" && exit 1)
	@if [ -f "coverage/cobertura.xml" ]; then \
		OVERALL_COVERAGE=$$(grep -oP 'line-rate="\K[0-9.]+' coverage/cobertura.xml | head -1); \
		COVERAGE_PERCENT=$$(echo "$$OVERALL_COVERAGE * 100" | bc); \
		echo "Overall coverage: $${COVERAGE_PERCENT}%"; \
		if [ $$(echo "$$OVERALL_COVERAGE < 0.80" | bc -l) -eq 1 ]; then \
			echo "❌ Overall coverage $${COVERAGE_PERCENT}% is below required 80%"; \
			exit 1; \
		fi; \
		echo "✅ Overall coverage $${COVERAGE_PERCENT}% meets requirement (≥80%)"; \
	else \
		echo "⚠️  Coverage file not found, skipping coverage check"; \
	fi
	@echo "📊 Coverage report: coverage/index.html"

pr-checks:
	@echo "🚀 Running all PR validation checks..."
	@echo ""
	@echo "Step 1/4: Code Quality Checks"
	@echo "=============================="
	@$(MAKE) code-quality-frontend
	@$(MAKE) code-quality-rust
	@echo ""
	@echo "Step 2/4: Build Validation"
	@echo "=============================="
	@$(MAKE) build
	@echo ""
	@echo "Step 3/4: Test Coverage"
	@echo "=============================="
	@$(MAKE) coverage-rust
	@echo ""
	@echo "=============================="
	@echo "✅ All PR checks passed!"
	@echo "=============================="
	@echo ""
	@echo "📋 Summary:"
	@echo "  ✅ Frontend code quality checks passed"
	@echo "  ✅ Rust code quality checks passed"
	@echo "  ✅ Frontend builds successfully"
	@echo "  ✅ Tauri backend builds successfully"
	@echo "  ✅ Rust test coverage ≥ 80%"
	@echo ""
	@echo "🎉 Your PR is ready for submission!"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@rm -rf dist/
	@rm -rf src-tauri/target/
	@rm -rf coverage/
	@rm -rf node_modules/.cache/
	@echo "✅ Cleaned successfully"

