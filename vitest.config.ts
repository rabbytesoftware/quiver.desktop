import { configDefaults, defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
    test: {
        environment: 'jsdom',
        globals: true,
        // `e2e/` is a SEPARATE suite with its own runner, its own package.json
        // and its own node_modules (see e2e/README.md): the scenarios there are
        // WebdriverIO specs that drive a real built app through tauri-driver,
        // and they rely on the `browser`/`$` globals only that runner injects.
        // Without this, vitest's default include glob
        // (`**/*.{test,spec}.?(c|m)[jt]s?(x)`) picks `e2e/scenarios/*.spec.ts`
        // up into the unit run, where they load under jsdom with no such
        // globals and fail. Spread rather than replace `configDefaults.exclude`
        // — node_modules/dist/etc. still need excluding, and dropping them also
        // drags `e2e/node_modules/**` into collection.
        //
        // `scripts/resolve-core-version.test.mjs` is deliberately NOT covered
        // here: it is a real unit test at the repo root and must keep running.
        exclude: [...configDefaults.exclude, 'e2e/**'],
        setupFiles: [
            '@testing-library/jest-dom/vitest',
            './src/__mocks__/setup-local-storage.ts',
            './src/__mocks__/setup-pointer-capture.ts',
            './src/__mocks__/setup-resize-observer.ts',
            './src/__mocks__/setup-element-animations.ts',
        ],
        coverage: {
            provider: 'v8',
            reporter: ['text', 'lcov', 'html'],
            reportsDirectory: './coverage',
            thresholds: {
                lines: 95,
                functions: 95,
                branches: 95,
                statements: 95,
            },
            include: ['src/**/*.ts', 'src/**/*.tsx'],
            exclude: [
                'src/routeTree.gen.ts',
                'src/main.tsx',
                'src/routes/**',
                'src/**/*.test.ts',
                'src/**/*.test.tsx',
                'src/__mocks__/**',
                'src/**/{store,mutations}/index.ts',
                // Vendored: `shadcn add` writes these from the registry, and
                // `shadcn diff` is what checks them. Same reasoning as
                // routeTree.gen.ts — testing them tests upstream's code, and
                // rewriting them to be testable is how you lose the ability to
                // pull an upstream fix.
                'src/components/ui/**',
            ],
        },
    },
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
});
