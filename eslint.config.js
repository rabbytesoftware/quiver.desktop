import js from '@eslint/js';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import reactPlugin from 'eslint-plugin-react';
import jsxA11yPlugin from 'eslint-plugin-jsx-a11y';
import importPlugin from 'eslint-plugin-import';

export default [
	js.configs.recommended,
	{
		files: ['**/*.{ts,tsx,js,jsx}'],
		languageOptions: {
			parser: tsParser,
			parserOptions: {
				ecmaVersion: 'latest',
				sourceType: 'module',
				ecmaFeatures: {
					jsx: true,
				},
			},
			globals: {
				console: 'readonly',
				window: 'readonly',
				document: 'readonly',
				navigator: 'readonly',
				setTimeout: 'readonly',
				setInterval: 'readonly',
				clearTimeout: 'readonly',
				clearInterval: 'readonly',
				Promise: 'readonly',
				fetch: 'readonly',
			},
		},
		plugins: {
			'@typescript-eslint': tsPlugin,
			'react': reactPlugin,
			'jsx-a11y': jsxA11yPlugin,
			'import': importPlugin,
		},
		rules: {
			...tsPlugin.configs.recommended.rules,
			...reactPlugin.configs.recommended.rules,
			...jsxA11yPlugin.configs.recommended.rules,

			'react/react-in-jsx-scope': 'off',
			'react/jsx-uses-react': 'off',

			'no-undef': 'off',

			'brace-style': 'off',
			'space-in-parens': 'off',
			'keyword-spacing': 'off',
			'space-infix-ops': 'off',
			'comma-spacing': 'off',
			'object-curly-spacing': 'off',
			'array-bracket-spacing': 'off',

			'@typescript-eslint/no-unused-vars': ['error', {
				argsIgnorePattern: '^_',
				varsIgnorePattern: '^_',
			}],

			'jsx-a11y/label-has-associated-control': ['warn', {
				assert: 'either',
				depth: 3,
			}],
			'jsx-a11y/no-static-element-interactions': 'warn',

			'import/order': ['error', {
				'groups': [
					['builtin', 'external'],
					['internal'],
					['parent', 'sibling', 'index'],
				],
				'pathGroups': [
					{
						pattern: 'react',
						group: 'builtin',
						position: 'before',
					},
					{
						pattern: 'preact',
						group: 'builtin',
						position: 'before',
					},
					{
						pattern: 'next/**',
						group: 'builtin',
						position: 'before',
					},
					{
						pattern: '@tauri-apps/**',
						group: 'external',
						position: 'after',
					},
					{
						pattern: '@/components/**',
						group: 'internal',
						position: 'before',
					},
					{
						pattern: '@radix-ui/**',
						group: 'internal',
						position: 'after',
					},
					{
						pattern: '@base-ui-components/**',
						group: 'internal',
						position: 'after',
					},
					{
						pattern: 'lucide-react',
						group: 'parent',
						position: 'before',
					},
					{
						pattern: '@/components/icons/**',
						group: 'parent',
						position: 'before',
					},
				],
				'pathGroupsExcludedImportTypes': ['react', 'preact'],
				'newlines-between': 'always',
				'alphabetize': {
					order: 'asc',
					caseInsensitive: true,
				},
			}],

			'jsx-a11y/alt-text': 'error',
			'jsx-a11y/aria-props': 'error',
			'jsx-a11y/aria-proptypes': 'error',
			'jsx-a11y/aria-unsupported-elements': 'error',
			'jsx-a11y/role-has-required-aria-props': 'error',
			'jsx-a11y/role-supports-aria-props': 'error',
		},
		settings: {
			react: {
				version: 'detect',
				pragma: 'h',
			},
			'import/resolver': {
				typescript: {
					alwaysTryTypes: true,
					project: './tsconfig.json',
				},
				node: {
					extensions: ['.js', '.jsx', '.ts', '.tsx'],
				},
			},
		},
	},
	{
		ignores: [
			'dist/',
			'build/',
			'node_modules/',
			'src-tauri/target/',
			'*.config.js',
			'*.config.ts',
		],
	},
];

