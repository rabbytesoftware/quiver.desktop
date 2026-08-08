import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
    test: {
        environment: 'jsdom',
        globals: true,
        setupFiles: [
            '@testing-library/jest-dom/vitest',
            './src/__mocks__/setup-local-storage.ts',
            './src/__mocks__/setup-pointer-capture.ts',
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
