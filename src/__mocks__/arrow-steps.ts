import type { ArrowStepDefinition, SignalKind } from '@/domain/arrow';

/**
 * Minimal-but-real `ArrowStepDefinition` builders for tests that only care
 * about `type`/`title` -- the domain type carries a full raw step definition
 * (command/url/checksum/signal, each per quiver.core's actual wire shape),
 * and most tests have no reason to spell all of that out inline every time.
 */

type RunFields = Omit<Extract<ArrowStepDefinition, { type: 'run' }>, 'type' | 'title'>;
type FetchFields = Omit<Extract<ArrowStepDefinition, { type: 'fetch' }>, 'type' | 'title'>;
type SignalFields = Omit<Extract<ArrowStepDefinition, { type: 'signal' }>, 'type' | 'title'>;

export function runStep(title: string, overrides: Partial<RunFields> = {}): ArrowStepDefinition {
	return { type: 'run', title, command: 'echo hello', elevated: false, timeout: '30s', ...overrides };
}

export function fetchStep(title: string, overrides: Partial<FetchFields> = {}): ArrowStepDefinition {
	return {
		type: 'fetch',
		title,
		url: 'https://example.com/archive.tar.gz',
		to: '{{workdir}}/archive.tar.gz',
		checksum: '',
		timeout: '60s',
		...overrides,
	};
}

export function signalStep(title: string, overrides: Partial<SignalFields> = {}): ArrowStepDefinition {
	const signal: SignalKind = 'graceful';
	return { type: 'signal', title, signal, timeout: '10s', ...overrides };
}

export function dependenciesStep(title: string): ArrowStepDefinition {
	return { type: 'dependencies', title };
}
