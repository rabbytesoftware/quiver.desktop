# Foundation & CI/CD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate from Preact to React, set up TanStack Router/Query, configure Tauri sidecar plugin, type the API envelope, and build CI/release workflows with auto-versioning.

**Architecture:** React frontend with file-based TanStack Router, TanStack Query for server state, Tauri shell plugin for sidecar management. CI validates code quality and build on PRs. Release workflow builds multi-platform Tauri bundles with quiver daemon sidecar, triggered by release branch PRs. Merge-to-master handles backport and final release.

**Tech Stack:** React 19, TanStack Router, TanStack Query, Tauri v2, tauri-plugin-shell, Vite, Bun, GitHub Actions

**Spec:** `docs/superpowers/specs/2026-04-21-foundation-and-ci-design.md`

---

### Task 1: Preact to React Migration

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `tsconfig.json`
- Modify: `src/main.tsx`
- Modify: `src/components/quiver/typography.tsx`
- Modify: `eslint.config.js`
- Modify: `components.json`
- Delete: `src/App.tsx` (will be replaced by router in Task 2)
- Delete: `src/App.css` → Move to `src/index.css`

- [ ] **Step 1: Remove Preact packages and install React**

```bash
bun remove preact @preact/preset-vite
bun add react react-dom
bun add -d @vitejs/plugin-react @types/react @types/react-dom
```

- [ ] **Step 2: Update `vite.config.ts` — remove Preact, add React plugin, remove compat aliases**

Replace the entire file with:

```ts
import path from "path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST

// https://vitejs.dev/config/
export default defineConfig(async () => ({
	plugins: [react(), tailwindcss()],
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: "ws",
					host,
					port: 1421,
			  }
			: undefined,
		watch: {
			ignored: ["**/src-tauri/**"],
		},
	},
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
		},
	},
}))
```

- [ ] **Step 3: Update `tsconfig.json` — remove Preact JSX source and path aliases**

Replace the entire file with:

```json
{
	"compilerOptions": {
		"target": "ES2020",
		"useDefineForClassFields": true,
		"module": "ESNext",
		"lib": ["ES2020", "DOM", "DOM.Iterable"],
		"skipLibCheck": true,

		"moduleResolution": "bundler",
		"allowImportingTsExtensions": true,
		"resolveJsonModule": true,
		"isolatedModules": true,
		"noEmit": true,
		"jsx": "react-jsx",

		"strict": true,
		"noUnusedLocals": true,
		"noUnusedParameters": true,
		"noFallthroughCasesInSwitch": true,

		"baseUrl": ".",
		"paths": {
			"@/*": ["./src/*"]
		}
	},
	"include": ["src"],
	"references": [
		{
			"path": "./tsconfig.node.json"
		}
	]
}
```

- [ ] **Step 4: Update `src/main.tsx` — use React DOM createRoot**

Replace entire file:

```tsx
import React from "react"
import ReactDOM from "react-dom/client"

import App from "./App"

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<App />
	</React.StrictMode>,
)
```

- [ ] **Step 5: Update `src/components/quiver/typography.tsx` — replace Preact types with React types**

Replace all `Preact.ComponentChildren` with `React.ReactNode` and remove the `import Preact from 'preact'` line. Replace with:

```tsx
import type { ReactNode } from "react"

export function Header1({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return (
		<h1 className={`scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl ${fontClass} ${className}`}>
			{children}
		</h1>
	)
}

export function Header2({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return (
		<h2
			className={`${fontClass} scroll-m-20 border-b border-dashed pb-2 mb-2 text-3xl font-semibold tracking-tight first:mt-0 ${className}`}
		>
			{children}
		</h2>
	)
}

export function Header3({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return (
		<h3 className={`scroll-m-20 text-2xl font-semibold tracking-tight ${fontClass} ${className}`}>{children}</h3>
	)
}

export function Header4({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return <h4 className={`${fontClass} scroll-m-20 text-xl font-semibold tracking-tight ${className}`}>{children}</h4>
}

export function Paragraph({
	children,
	className,
	useSecondaryFont = false,
}: {
	children: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return <div className={`leading-7 not-first:mt-6 ${fontClass} ${className}`}>{children}</div>
}

export function Span({
	children,
	className,
	useSecondaryFont = false,
}: {
	children?: ReactNode
	className?: string
	useSecondaryFont?: boolean
}) {
	const fontClass = useSecondaryFont ? "" : ""
	return <span className={`${fontClass} ${className}`}>{children}</span>
}
```

- [ ] **Step 6: Update `eslint.config.js` — remove Preact references**

Remove the `preact` pathGroup entry from the `import/order` rule (lines 82-86). Remove `'preact'` from `pathGroupsExcludedImportTypes`. Remove the `pragma: 'h'` from react settings. The react settings should become:

```js
settings: {
	react: {
		version: 'detect',
	},
```

- [ ] **Step 7: Rename `src/App.css` to `src/index.css` and update imports**

Rename the file. Update `src/App.tsx` to import `./index.css` instead of `./App.css`. Also update `components.json` to point to the new CSS location:

In `components.json`, change:
```json
"css": "src/App.css"
```
to:
```json
"css": "src/index.css"
```

- [ ] **Step 8: Verify the migration builds**

```bash
bun run build
```

Expected: Successful build with no errors. The `dist/` folder should be created.

- [ ] **Step 9: Verify Rust build still works**

```bash
cd src-tauri && cargo build
```

Expected: Successful compilation (Rust side is unchanged).

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: migrate from Preact to React

Remove Preact and compat aliases, install React 19 with
@vitejs/plugin-react. Update all component types from
Preact.ComponentChildren to React.ReactNode."
```

---

### Task 2: TanStack Router

**Files:**
- Modify: `package.json` (via bun add)
- Modify: `vite.config.ts`
- Create: `src/routes/__root.tsx`
- Create: `src/routes/index.tsx`
- Create: `src/routes/quivers/index.tsx`
- Create: `src/routes/quivers/$namespace.tsx`
- Create: `src/routes/arrows/index.tsx`
- Create: `src/routes/arrows/$namespace.tsx`
- Modify: `src/main.tsx`
- Delete: `src/App.tsx` (replaced by root route)

- [ ] **Step 1: Install TanStack Router packages**

```bash
bun add @tanstack/react-router
bun add -d @tanstack/router-plugin @tanstack/react-router-devtools
```

- [ ] **Step 2: Add TanStack Router Vite plugin to `vite.config.ts`**

Add the import at the top:

```ts
import { TanStackRouterVite } from "@tanstack/router-plugin/vite"
```

Add it to the plugins array (must come before `react()`):

```ts
plugins: [TanStackRouterVite({ quoteStyle: "double" }), react(), tailwindcss()],
```

- [ ] **Step 3: Create `src/routes/__root.tsx` — root layout**

```tsx
import { createRootRoute, Outlet } from "@tanstack/react-router"
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools"

import "../index.css"

export const Route = createRootRoute({
	component: RootLayout,
})

function RootLayout() {
	return (
		<>
			<Outlet />
			<TanStackRouterDevtools position="bottom-right" />
		</>
	)
}
```

- [ ] **Step 4: Create `src/routes/index.tsx` — home page**

```tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/")({
	component: HomePage,
})

function HomePage() {
	return (
		<main className="p-4">
			<h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl">
				Quiver Desktop
			</h1>
		</main>
	)
}
```

- [ ] **Step 5: Create `src/routes/quivers/index.tsx` — quiver list page**

```tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/quivers/")({
	component: QuiversPage,
})

function QuiversPage() {
	return (
		<main className="p-4">
			<h1 className="scroll-m-20 text-3xl font-semibold tracking-tight">
				Quivers
			</h1>
		</main>
	)
}
```

- [ ] **Step 6: Create `src/routes/quivers/$namespace.tsx` — quiver detail page**

```tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/quivers/$namespace")({
	component: QuiverDetailPage,
})

function QuiverDetailPage() {
	const { namespace } = Route.useParams()
	return (
		<main className="p-4">
			<h1 className="scroll-m-20 text-3xl font-semibold tracking-tight">
				Quiver: {namespace}
			</h1>
		</main>
	)
}
```

- [ ] **Step 7: Create `src/routes/arrows/index.tsx` — arrow list page**

```tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/arrows/")({
	component: ArrowsPage,
})

function ArrowsPage() {
	return (
		<main className="p-4">
			<h1 className="scroll-m-20 text-3xl font-semibold tracking-tight">
				Arrows
			</h1>
		</main>
	)
}
```

- [ ] **Step 8: Create `src/routes/arrows/$namespace.tsx` — arrow detail page**

```tsx
import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/arrows/$namespace")({
	component: ArrowDetailPage,
})

function ArrowDetailPage() {
	const { namespace } = Route.useParams()
	return (
		<main className="p-4">
			<h1 className="scroll-m-20 text-3xl font-semibold tracking-tight">
				Arrow: {namespace}
			</h1>
		</main>
	)
}
```

- [ ] **Step 9: Update `src/main.tsx` — mount the router**

Replace the entire file:

```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import { RouterProvider, createRouter } from "@tanstack/react-router"

import { routeTree } from "./routeTree.gen"

const router = createRouter({ routeTree })

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<RouterProvider router={router} />
	</React.StrictMode>,
)
```

- [ ] **Step 10: Delete `src/App.tsx`**

This file is no longer needed — the root route replaces it.

```bash
rm src/App.tsx
```

- [ ] **Step 11: Generate route tree and verify build**

```bash
bun run build
```

Expected: The router plugin generates `src/routeTree.gen.ts` during build, and the build succeeds.

- [ ] **Step 12: Add `src/routeTree.gen.ts` to eslint ignores**

In `eslint.config.js`, add to the ignores array:

```js
{
	ignores: [
		'dist/',
		'build/',
		'node_modules/',
		'src-tauri/target/',
		'*.config.js',
		'*.config.ts',
		'src/routeTree.gen.ts',
	],
},
```

- [ ] **Step 13: Commit**

```bash
git add -A
git commit -m "feat: add TanStack Router with file-based routing

Set up type-safe file-based routing with TanStack Router.
Routes: /, /quivers, /quivers/:namespace, /arrows, /arrows/:namespace.
Includes devtools and auto-generated route tree."
```

---

### Task 3: TanStack Query + API Types + Client

**Files:**
- Modify: `package.json` (via bun add)
- Create: `src/lib/api/types.ts`
- Create: `src/lib/api/client.ts`
- Create: `src/lib/api/query-keys.ts`
- Modify: `src/main.tsx`

- [ ] **Step 1: Install TanStack Query**

```bash
bun add @tanstack/react-query
bun add -d @tanstack/react-query-devtools
```

- [ ] **Step 2: Create `src/lib/api/types.ts` — API response envelope**

```ts
/** Standard envelope for all quiver.core responses */
export interface ApiResponse<T = unknown> {
	success: boolean
	error: string | null
	data?: T
}

/** Mutation responses return namespace instead of data */
export interface ApiMutationResponse {
	success: boolean
	error: string | null
	namespace: string
}
```

- [ ] **Step 3: Create `src/lib/api/client.ts` — fetch wrapper**

```ts
import type { ApiResponse } from "./types"

const DEFAULT_PORT = 6982
const BASE_URL = `http://localhost:${DEFAULT_PORT}`

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string,
	) {
		super(message)
		this.name = "ApiError"
	}
}

export async function api<T>(
	path: string,
	options?: RequestInit,
): Promise<T> {
	const res = await fetch(`${BASE_URL}${path}`, {
		...options,
		headers: {
			"Content-Type": "application/json",
			...options?.headers,
		},
	})

	const envelope: ApiResponse<T> = await res.json()

	if (!envelope.success) {
		throw new ApiError(res.status, envelope.error ?? "Unknown error")
	}

	return envelope.data as T
}
```

- [ ] **Step 4: Create `src/lib/api/query-keys.ts` — query key factories**

```ts
export const arrowKeys = {
	all: ["arrows"] as const,
	lists: () => [...arrowKeys.all, "list"] as const,
	detail: (namespace: string) => [...arrowKeys.all, "detail", namespace] as const,
}

export const quiverKeys = {
	all: ["quivers"] as const,
	lists: () => [...quiverKeys.all, "list"] as const,
	detail: (namespace: string) => [...quiverKeys.all, "detail", namespace] as const,
}
```

- [ ] **Step 5: Update `src/main.tsx` — add QueryClientProvider**

Replace the entire file:

```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ReactQueryDevtools } from "@tanstack/react-query-devtools"
import { RouterProvider, createRouter } from "@tanstack/react-router"

import { routeTree } from "./routeTree.gen"

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 30_000,
			retry: 1,
		},
	},
})

const router = createRouter({ routeTree })

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
			<ReactQueryDevtools initialIsOpen={false} />
		</QueryClientProvider>
	</React.StrictMode>,
)
```

- [ ] **Step 6: Verify build**

```bash
bun run build
```

Expected: Successful build.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add TanStack Query, API types, and fetch client

Set up QueryClientProvider with sensible defaults (30s stale, 1 retry).
Add typed API response envelope matching quiver.core contract.
Add fetch wrapper and query key factories for arrows and quivers."
```

---

### Task 4: Tauri Sidecar Plugin

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json` (via bun add)
- Create: `src/lib/sidecar.ts`
- Create: `src-tauri/binaries/.gitkeep`
- Modify: `.gitignore`

- [ ] **Step 1: Add `tauri-plugin-shell` to Rust dependencies**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
tauri-plugin-shell = "2"
```

- [ ] **Step 2: Register the shell plugin in `src-tauri/src/lib.rs`**

Replace the entire file:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Note: The `greet` command is removed — it was scaffolding placeholder.

- [ ] **Step 3: Configure sidecar in `src-tauri/tauri.conf.json`**

Add `externalBin` to the `bundle` section:

```json
"bundle": {
	"active": true,
	"targets": "all",
	"externalBin": [
		"binaries/quiver"
	],
	"icon": [
		"icons/32x32.png",
		"icons/128x128.png",
		"icons/128x128@2x.png",
		"icons/icon.icns",
		"icons/icon.ico"
	]
}
```

- [ ] **Step 4: Update capabilities in `src-tauri/capabilities/default.json`**

```json
{
	"$schema": "../gen/schemas/desktop-schema.json",
	"identifier": "default",
	"description": "Capability for the main window",
	"windows": ["main"],
	"permissions": [
		"core:default",
		"opener:default",
		"shell:allow-spawn",
		"shell:allow-kill"
	]
}
```

- [ ] **Step 5: Install `@tauri-apps/plugin-shell` npm package**

```bash
bun add @tauri-apps/plugin-shell
```

- [ ] **Step 6: Create `src/lib/sidecar.ts` — sidecar manager**

```ts
import { Command } from "@tauri-apps/plugin-shell"

const DEFAULT_PORT = 6982

let sidecarProcess: Awaited<ReturnType<Command["spawn"]>> | null = null

export async function startDaemon(port: number = DEFAULT_PORT): Promise<void> {
	if (sidecarProcess) {
		return
	}

	const command = Command.sidecar("binaries/quiver", [
		"daemon",
		"--port",
		String(port),
	])

	command.on("error", (error) => {
		console.error("Sidecar error:", error)
		sidecarProcess = null
	})

	command.on("close", ({ code }) => {
		console.log("Sidecar exited with code:", code)
		sidecarProcess = null
	})

	sidecarProcess = await command.spawn()
}

export async function stopDaemon(): Promise<void> {
	if (sidecarProcess) {
		await sidecarProcess.kill()
		sidecarProcess = null
	}
}
```

- [ ] **Step 7: Create `src-tauri/binaries/.gitkeep` and update `.gitignore`**

Create the binaries directory placeholder:

```bash
touch src-tauri/binaries/.gitkeep
```

Add to `.gitignore`:

```
# Sidecar binaries (fetched during release, not committed)
src-tauri/binaries/quiver-*
```

- [ ] **Step 8: Verify Rust build compiles with shell plugin**

```bash
cd src-tauri && cargo build
```

Expected: Successful compilation. There will be a warning about missing sidecar binary — this is expected for local dev without the binary.

- [ ] **Step 9: Verify frontend build**

```bash
bun run build
```

Expected: Successful build.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: configure Tauri sidecar plugin for quiver daemon

Add tauri-plugin-shell for spawning the quiver daemon as a sidecar.
Configure externalBin, shell capabilities, and sidecar manager module.
Binaries directory is gitignored — populated by CI/release workflows."
```

---

### Task 5: CI Workflow

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace `.github/workflows/ci.yml` with the new workflow**

```yaml
name: CI

on:
  pull_request:
    branches: [ master, develop ]
    types: [ opened, synchronize, reopened, ready_for_review ]

env:
  RUST_VERSION: '1.85'
  BUN_VERSION: 'latest'

jobs:
  draft-reminder:
    name: Draft PR Reminder
    runs-on: ubuntu-latest
    if: github.event.pull_request.draft == true && github.event.action == 'opened'
    permissions:
      pull-requests: write
    steps:
    - name: Comment on draft PR
      uses: actions/github-script@v9
      with:
        script: |
          github.rest.issues.createComment({
            issue_number: context.issue.number,
            owner: context.repo.owner,
            repo: context.repo.repo,
            body: `⚠️ **Draft PR Reminder**\n\nHey @${context.payload.pull_request.user.login}!\n\nBe sure to run the command \`make pr-checks\` before opening this PR. Thanks!\n\n> **Note:** CI won't run on draft PRs to optimize [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions). Mark as ready for review when you're done! 🚀`
          });

  validate-branch:
    name: Validate Branch Model
    runs-on: ubuntu-latest
    steps:
    - name: Check if PR is draft
      run: |
        if [ "${{ github.event.pull_request.draft }}" == "true" ]; then
          echo "❌ This PR is a draft. Please mark it as ready for review to run CI checks."
          exit 1
        fi
        echo "✅ PR is ready for review"

    - name: Checkout code
      uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - name: Validate branching model
      run: |
        SOURCE_BRANCH="${{ github.head_ref }}"
        TARGET_BRANCH="${{ github.base_ref }}"

        echo "Source branch: $SOURCE_BRANCH"
        echo "Target branch: $TARGET_BRANCH"

        validate_branch() {
          case "$SOURCE_BRANCH" in
            enhancement/*)
              if [ "$TARGET_BRANCH" != "develop" ]; then
                echo "❌ Enhancement branches can only target 'develop', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            feature/*)
              if [ "$TARGET_BRANCH" != "develop" ]; then
                echo "❌ Feature branches can only target 'develop', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            fix/*)
              if [ "$TARGET_BRANCH" != "develop" ]; then
                echo "❌ Fix branches can only target 'develop', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            refactor/*)
              if [ "$TARGET_BRANCH" != "develop" ]; then
                echo "❌ Refactor branches can only target 'develop', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            dependabot/*)
              if [ "$TARGET_BRANCH" != "develop" ]; then
                echo "❌ Dependabot branches can only target 'develop', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            hotfix/*)
              if [ "$TARGET_BRANCH" != "master" ]; then
                echo "❌ Hotfix branches can only target 'master', not '$TARGET_BRANCH'"
                exit 1
              fi
              ;;
            release/*)
              if [ "$TARGET_BRANCH" != "master" ]; then
                echo "❌ Release branches can only target 'master', not '$TARGET_BRANCH'"
                exit 1
              fi
              MERGE_BASE=$(git merge-base origin/develop HEAD)
              DEVELOP_HEAD=$(git rev-parse origin/develop)
              if [ "$MERGE_BASE" != "$DEVELOP_HEAD" ]; then
                echo "❌ Release branches must be created from the latest 'develop' branch"
                exit 1
              fi
              ;;
            *)
              echo "❌ Invalid branch name '$SOURCE_BRANCH'. Must follow pattern:"
              echo "  - enhancement/name (→ develop)"
              echo "  - feature/name (→ develop)"
              echo "  - fix/name (→ develop)"
              echo "  - refactor/name (→ develop)"
              echo "  - hotfix/name (→ master)"
              echo "  - release/yyyy-mm-dd (develop → master)"
              exit 1
              ;;
          esac

          echo "✅ Quiver loves your PR: $SOURCE_BRANCH → $TARGET_BRANCH"
        }

        validate_branch

  code-quality-frontend:
    name: Code Quality - Frontend
    runs-on: ubuntu-latest
    needs: validate-branch

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Set up Bun
      uses: oven-sh/setup-bun@v2
      with:
        bun-version: ${{ env.BUN_VERSION }}

    - name: Cache Bun dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.bun/install/cache
          node_modules
        key: ${{ runner.os }}-bun-${{ hashFiles('bun.lock') }}
        restore-keys: |
          ${{ runner.os }}-bun-

    - name: Install dependencies
      run: bun install

    - name: Format check
      run: |
        bunx prettier --check "src/**/*.{ts,tsx,js,jsx}"
        echo "✅ Code formatting is correct"

    - name: Lint check
      run: |
        bunx eslint "src/**/*.{ts,tsx,js,jsx}"
        echo "✅ Linting passed"

    - name: Type checking
      run: |
        bunx tsc --noEmit
        echo "✅ Type checking passed"

  code-quality-rust:
    name: Code Quality - Rust
    runs-on: ubuntu-latest
    needs: validate-branch

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Install system dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libwebkit2gtk-4.1-dev \
          build-essential \
          curl \
          wget \
          file \
          libssl-dev \
          libayatana-appindicator3-dev \
          librsvg2-dev

    - name: Set up Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: ${{ env.RUST_VERSION }}
        components: rustfmt, clippy

    - name: Cache Cargo dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          src-tauri/target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-

    - name: Format check
      run: |
        cd src-tauri
        cargo fmt --check
        echo "✅ Rust formatting is correct"

    - name: Clippy check
      run: |
        cd src-tauri
        cargo clippy -- -D warnings
        echo "✅ Clippy checks passed"

    - name: Security audit
      run: |
        cargo install cargo-audit --locked --force
        cd src-tauri
        cargo audit
        echo "✅ Security audit passed"

  build-frontend:
    name: Build - Frontend
    runs-on: ubuntu-latest
    needs: [code-quality-frontend]

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Set up Bun
      uses: oven-sh/setup-bun@v2
      with:
        bun-version: ${{ env.BUN_VERSION }}

    - name: Cache Bun dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.bun/install/cache
          node_modules
        key: ${{ runner.os }}-bun-${{ hashFiles('bun.lock') }}
        restore-keys: |
          ${{ runner.os }}-bun-

    - name: Install dependencies
      run: bun install

    - name: Build frontend
      run: bun run build

    - name: Verify build output
      run: |
        if [ ! -d "dist" ]; then
          echo "❌ dist/ folder not found after build"
          exit 1
        fi
        echo "✅ Frontend built successfully"
        ls -la dist/

  build-tauri:
    name: Build - Tauri
    runs-on: ubuntu-latest
    needs: [code-quality-rust]

    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Install system dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libwebkit2gtk-4.1-dev \
          build-essential \
          curl \
          wget \
          file \
          libssl-dev \
          libayatana-appindicator3-dev \
          librsvg2-dev

    - name: Set up Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: ${{ env.RUST_VERSION }}

    - name: Set up Bun
      uses: oven-sh/setup-bun@v2
      with:
        bun-version: ${{ env.BUN_VERSION }}

    - name: Cache Cargo dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          src-tauri/target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-

    - name: Install frontend dependencies
      run: bun install

    - name: Build Tauri backend
      run: |
        cd src-tauri
        cargo build --release

    - name: Verify binary
      run: |
        BINARY="src-tauri/target/release/quiverdesktop"
        if [ ! -f "$BINARY" ]; then
          echo "❌ Binary not found at $BINARY after build"
          exit 1
        fi
        echo "✅ Tauri binary built successfully"
```

- [ ] **Step 2: Validate YAML syntax**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "✅ Valid YAML"
```

If python3/yaml not available, use:

```bash
bun -e "const fs = require('fs'); console.log('✅ File exists:', fs.existsSync('.github/workflows/ci.yml'))"
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "feat: rewrite CI workflow matching quiver.core patterns

Draft PR reminders, branch validation, frontend quality checks
(format, lint, types), Rust quality checks (fmt, clippy, audit),
and separate frontend/Tauri build validation jobs."
```

---

### Task 6: Release Workflow

**Files:**
- Create: `.github/actions/build-tauri/action.yml`
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/actions/build-tauri/action.yml` — shared composite action**

```bash
mkdir -p .github/actions/build-tauri
```

```yaml
name: 'Build Tauri App'
description: 'Fetch quiver sidecar and build Tauri app for the current platform'

inputs:
  quiver-version:
    description: 'Quiver core release version tag (e.g. v0.1.0)'
    required: true
  rust-version:
    description: 'Rust toolchain version'
    required: false
    default: '1.85'
  bun-version:
    description: 'Bun version'
    required: false
    default: 'latest'

runs:
  using: 'composite'
  steps:
    - name: Determine platform and sidecar name
      id: platform
      shell: bash
      run: |
        case "${{ runner.os }}" in
          Linux)
            echo "quiver_binary=quiver-linux-amd64" >> $GITHUB_OUTPUT
            echo "target_triple=x86_64-unknown-linux-gnu" >> $GITHUB_OUTPUT
            echo "extension=" >> $GITHUB_OUTPUT
            ;;
          macOS)
            echo "quiver_binary=quiver-darwin-arm64" >> $GITHUB_OUTPUT
            echo "target_triple=aarch64-apple-darwin" >> $GITHUB_OUTPUT
            echo "extension=" >> $GITHUB_OUTPUT
            ;;
          Windows)
            echo "quiver_binary=quiver-windows-amd64.exe" >> $GITHUB_OUTPUT
            echo "target_triple=x86_64-pc-windows-msvc" >> $GITHUB_OUTPUT
            echo "extension=.exe" >> $GITHUB_OUTPUT
            ;;
        esac

    - name: Install system dependencies (Linux)
      if: runner.os == 'Linux'
      shell: bash
      run: |
        sudo apt-get update
        sudo apt-get install -y libwebkit2gtk-4.1-dev \
          build-essential curl wget file \
          libssl-dev libayatana-appindicator3-dev librsvg2-dev

    - name: Set up Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: ${{ inputs.rust-version }}

    - name: Set up Bun
      uses: oven-sh/setup-bun@v2
      with:
        bun-version: ${{ inputs.bun-version }}

    - name: Cache Cargo dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          src-tauri/target/
        key: ${{ runner.os }}-cargo-release-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-release-

    - name: Fetch quiver sidecar binary
      shell: bash
      env:
        GH_TOKEN: ${{ github.token }}
      run: |
        mkdir -p src-tauri/binaries
        gh release download "${{ inputs.quiver-version }}" \
          --repo rabbytesoftware/quiver \
          --pattern "${{ steps.platform.outputs.quiver_binary }}" \
          --dir src-tauri/binaries

        SIDECAR_NAME="quiver-${{ steps.platform.outputs.target_triple }}${{ steps.platform.outputs.extension }}"
        mv "src-tauri/binaries/${{ steps.platform.outputs.quiver_binary }}" \
           "src-tauri/binaries/${SIDECAR_NAME}"
        chmod +x "src-tauri/binaries/${SIDECAR_NAME}"

        echo "✅ Sidecar ready: ${SIDECAR_NAME}"
        ls -la src-tauri/binaries/

    - name: Install frontend dependencies
      shell: bash
      run: bun install

    - name: Build frontend
      shell: bash
      run: bun run build

    - name: Install Tauri CLI
      shell: bash
      run: cargo install tauri-cli --version "^2" --locked

    - name: Build Tauri app
      shell: bash
      run: cargo tauri build
```

- [ ] **Step 2: Create `.github/workflows/release.yml`**

```yaml
name: Release

on:
  workflow_run:
    workflows: ["CI"]
    types: [completed]
    branches: ["release/**"]

  workflow_dispatch:
    inputs:
      version:
        description: 'Explicit version (e.g. v26.04.01). Leave empty for auto-increment.'
        required: false
      quiver_version:
        description: 'Quiver core release tag (e.g. v0.1.0)'
        required: true
      is_rc:
        description: 'Build as release candidate'
        type: boolean
        default: true

permissions:
  contents: write

jobs:
  # Gate: only run if CI passed (for workflow_run trigger)
  check-ci:
    name: Verify CI Status
    runs-on: ubuntu-latest
    if: >
      github.event_name == 'workflow_dispatch' ||
      (github.event_name == 'workflow_run' && github.event.workflow_run.conclusion == 'success')
    outputs:
      should_run: ${{ steps.check.outputs.should_run }}
      pr_branch: ${{ steps.check.outputs.pr_branch }}
    steps:
    - name: Check trigger
      id: check
      run: |
        echo "should_run=true" >> $GITHUB_OUTPUT
        if [ "${{ github.event_name }}" == "workflow_run" ]; then
          echo "pr_branch=${{ github.event.workflow_run.head_branch }}" >> $GITHUB_OUTPUT
        else
          echo "pr_branch=" >> $GITHUB_OUTPUT
        fi

  compute-version:
    name: Compute Version
    runs-on: ubuntu-latest
    needs: check-ci
    if: needs.check-ci.outputs.should_run == 'true'
    outputs:
      version: ${{ steps.version.outputs.version }}
      quiver_version: ${{ steps.inputs.outputs.quiver_version }}
    steps:
    - name: Checkout code
      uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - name: Resolve inputs
      id: inputs
      run: |
        if [ "${{ github.event_name }}" == "workflow_dispatch" ]; then
          echo "quiver_version=${{ github.event.inputs.quiver_version }}" >> $GITHUB_OUTPUT
        else
          # For automated runs, read from pinned config
          if [ -f ".quiver-version" ]; then
            echo "quiver_version=$(cat .quiver-version)" >> $GITHUB_OUTPUT
          else
            echo "❌ No .quiver-version file found and no quiver_version input provided"
            exit 1
          fi
        fi

    - name: Compute version
      id: version
      run: |
        # Use explicit version if provided
        if [ -n "${{ github.event.inputs.version }}" ]; then
          VERSION="${{ github.event.inputs.version }}"
          echo "Using explicit version: $VERSION"
          echo "version=$VERSION" >> $GITHUB_OUTPUT
          exit 0
        fi

        # Auto-increment from tags
        YY=$(date +%y)
        MM=$(date +%m)

        git fetch --tags
        LATEST=$(git tag --list "v${YY}.${MM}.*" --sort=-v:refname | grep -v '\-rc' | head -1)

        if [ -z "$LATEST" ]; then
          VERSION="v${YY}.${MM}.01"
        else
          XX=$(echo "$LATEST" | awk -F. '{print $3}')
          NEXT=$(printf "%02d" $((10#$XX + 1)))
          VERSION="v${YY}.${MM}.${NEXT}"
        fi

        # Append -rc.N if this is a release candidate
        IS_RC="${{ github.event.inputs.is_rc }}"
        if [ "$IS_RC" == "true" ] || [ "${{ github.event_name }}" == "workflow_run" ]; then
          # Count existing RCs for this version
          RC_COUNT=$(git tag --list "${VERSION}-rc.*" | wc -l | tr -d ' ')
          RC_NUM=$((RC_COUNT + 1))
          VERSION="${VERSION}-rc.${RC_NUM}"
        fi

        echo "Computed version: $VERSION"
        echo "version=$VERSION" >> $GITHUB_OUTPUT

  build-release:
    name: Build (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    needs: [compute-version]
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Build Tauri app
      uses: ./.github/actions/build-tauri
      with:
        quiver-version: ${{ needs.compute-version.outputs.quiver_version }}

    - name: Upload artifacts
      uses: actions/upload-artifact@v4
      with:
        name: tauri-build-${{ matrix.os }}
        path: |
          src-tauri/target/release/bundle/**/*.dmg
          src-tauri/target/release/bundle/**/*.app
          src-tauri/target/release/bundle/**/*.AppImage
          src-tauri/target/release/bundle/**/*.deb
          src-tauri/target/release/bundle/**/*.msi
          src-tauri/target/release/bundle/**/*.exe
        retention-days: 7

  publish-release:
    name: Publish Release
    runs-on: ubuntu-latest
    needs: [compute-version, build-release]
    steps:
    - name: Download all artifacts
      uses: actions/download-artifact@v4
      with:
        path: artifacts/
        merge-multiple: true

    - name: Determine release type
      id: release-type
      run: |
        VERSION="${{ needs.compute-version.outputs.version }}"
        if [[ "$VERSION" == *"-rc."* ]]; then
          echo "is_prerelease=true" >> $GITHUB_OUTPUT
          echo "draft=true" >> $GITHUB_OUTPUT
        else
          echo "is_prerelease=false" >> $GITHUB_OUTPUT
          echo "draft=false" >> $GITHUB_OUTPUT
        fi

    - name: Create GitHub Release
      uses: softprops/action-gh-release@v2
      with:
        tag_name: ${{ needs.compute-version.outputs.version }}
        name: Quiver Desktop ${{ needs.compute-version.outputs.version }}
        draft: ${{ steps.release-type.outputs.draft }}
        prerelease: ${{ steps.release-type.outputs.is_prerelease }}
        files: artifacts/**/*
        body: |
          ## Quiver Desktop ${{ needs.compute-version.outputs.version }}

          **Quiver Core:** ${{ needs.compute-version.outputs.quiver_version }}

          ### Downloads
          - **macOS:** `.dmg` installer
          - **Windows:** `.msi` installer
          - **Linux:** `.AppImage` or `.deb` package
        generate_release_notes: true
```

- [ ] **Step 3: Create `.quiver-version` file for pinned core version**

```
v0.1.0
```

This file is read by the release workflow when triggered automatically (not via manual dispatch).

- [ ] **Step 4: Commit**

```bash
git add .github/actions/build-tauri/action.yml .github/workflows/release.yml .quiver-version
git commit -m "feat: add release workflow with auto-versioning

Multi-platform Tauri builds (macOS, Windows, Linux) triggered by
CI success on release branches or manual dispatch. Auto-increments
YY.MM.XX version from tags. Shared build-tauri composite action
fetches quiver sidecar from core releases."
```

---

### Task 7: Merge-to-Master Workflow

**Files:**
- Create: `.github/workflows/merge-to-master.yml`

- [ ] **Step 1: Create `.github/workflows/merge-to-master.yml`**

```yaml
name: Merge to Master

on:
  pull_request:
    branches: [master]
    types: [closed]

permissions:
  contents: write
  pull-requests: write

jobs:
  backport-and-release:
    name: Backport & Final Release
    runs-on: ubuntu-latest
    if: >
      github.event.pull_request.merged == true &&
      startsWith(github.event.pull_request.head.ref, 'release/')
    outputs:
      version: ${{ steps.version.outputs.version }}
      quiver_version: ${{ steps.version.outputs.quiver_version }}
      backport_branch: ${{ steps.backport.outputs.branch }}
    steps:
    - name: Checkout code
      uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - name: Compute final version
      id: version
      run: |
        git fetch --tags

        # Find latest RC tag
        LATEST_RC=$(git tag --list "v*-rc.*" --sort=-v:refname | head -1)

        if [ -z "$LATEST_RC" ]; then
          echo "❌ No RC tags found. Cannot determine final version."
          exit 1
        fi

        # Strip -rc.N suffix to get final version
        VERSION=$(echo "$LATEST_RC" | sed 's/-rc\.[0-9]*//')
        echo "Final version: $VERSION (from RC: $LATEST_RC)"
        echo "version=$VERSION" >> $GITHUB_OUTPUT

        # Read quiver version
        if [ -f ".quiver-version" ]; then
          echo "quiver_version=$(cat .quiver-version)" >> $GITHUB_OUTPUT
        else
          echo "❌ No .quiver-version file found"
          exit 1
        fi

    - name: Create backport branch and PR
      id: backport
      env:
        GH_TOKEN: ${{ github.token }}
      run: |
        RELEASE_BRANCH="${{ github.event.pull_request.head.ref }}"
        VERSION="${{ steps.version.outputs.version }}"

        # Extract date from release branch name (release/YYYY-MM-DD)
        DATE=$(echo "$RELEASE_BRANCH" | sed 's|release/||')
        BACKPORT_BRANCH="backport/${DATE}-${VERSION}"

        echo "Creating backport branch: $BACKPORT_BRANCH"
        git checkout -b "$BACKPORT_BRANCH"
        git push origin "$BACKPORT_BRANCH"

        echo "Opening PR to develop..."
        gh pr create \
          --base develop \
          --head "$BACKPORT_BRANCH" \
          --title "Backport ${VERSION} to develop" \
          --body "Automated backport of release ${VERSION} from master to develop."

        echo "branch=$BACKPORT_BRANCH" >> $GITHUB_OUTPUT

  final-release:
    name: Final Release (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    needs: [backport-and-release]
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Build Tauri app
      uses: ./.github/actions/build-tauri
      with:
        quiver-version: ${{ needs.backport-and-release.outputs.quiver_version }}

    - name: Upload artifacts
      uses: actions/upload-artifact@v4
      with:
        name: final-build-${{ matrix.os }}
        path: |
          src-tauri/target/release/bundle/**/*.dmg
          src-tauri/target/release/bundle/**/*.app
          src-tauri/target/release/bundle/**/*.AppImage
          src-tauri/target/release/bundle/**/*.deb
          src-tauri/target/release/bundle/**/*.msi
          src-tauri/target/release/bundle/**/*.exe
        retention-days: 7

  publish-final:
    name: Publish Final Release
    runs-on: ubuntu-latest
    needs: [backport-and-release, final-release]
    steps:
    - name: Download all artifacts
      uses: actions/download-artifact@v4
      with:
        path: artifacts/
        pattern: final-build-*
        merge-multiple: true

    - name: Create GitHub Release
      uses: softprops/action-gh-release@v2
      with:
        tag_name: ${{ needs.backport-and-release.outputs.version }}
        name: Quiver Desktop ${{ needs.backport-and-release.outputs.version }}
        draft: false
        prerelease: false
        files: artifacts/**/*
        body: |
          ## Quiver Desktop ${{ needs.backport-and-release.outputs.version }}

          **Quiver Core:** ${{ needs.backport-and-release.outputs.quiver_version }}

          ### Downloads
          - **macOS:** `.dmg` installer
          - **Windows:** `.msi` installer
          - **Linux:** `.AppImage` or `.deb` package
        generate_release_notes: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/merge-to-master.yml
git commit -m "feat: add merge-to-master workflow for backport and final release

Auto-creates backport branch and PR to develop when release PRs
merge to master. Builds final release (non-RC) using shared
build-tauri composite action and publishes to GitHub Releases."
```

---

### Task 8: Final Verification

**Files:** None (verification only)

- [ ] **Step 1: Run full frontend build**

```bash
bun run build
```

Expected: Successful build, `dist/` created.

- [ ] **Step 2: Run Rust build**

```bash
cd src-tauri && cargo build
```

Expected: Successful compilation.

- [ ] **Step 3: Run type check**

```bash
bunx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 4: Run linter**

```bash
bunx eslint "src/**/*.{ts,tsx,js,jsx}"
```

Expected: No lint errors.

- [ ] **Step 5: Run formatter check**

```bash
bunx prettier --check "src/**/*.{ts,tsx,js,jsx}"
```

Expected: All files formatted correctly. If not, run `bunx prettier --write "src/**/*.{ts,tsx,js,jsx}"` and commit the fixes.

- [ ] **Step 6: Run Rust format and clippy**

```bash
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings
```

Expected: No formatting or lint issues.

- [ ] **Step 7: Validate all workflow YAML files**

```bash
for f in .github/workflows/*.yml; do echo "Checking $f..."; python3 -c "import yaml; yaml.safe_load(open('$f')); print('✅ Valid')"; done
```

- [ ] **Step 8: Final commit if any formatting fixes were needed**

Only if Steps 5-6 required changes:

```bash
git add -A
git commit -m "style: fix formatting issues from final verification"
```
