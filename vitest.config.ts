import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
    test: {
        environment: 'jsdom',
        globals: true,
        root: './src',
        setupFiles: ['@testing-library/jest-dom/vitest'],
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
                'src/**/*.test.ts',
                'src/**/*.test.tsx',
                'src/__mocks__/**',
            ],
        },
    },
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
});
