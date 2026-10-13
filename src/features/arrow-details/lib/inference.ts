import type { MessageKey } from '@/lib/i18n';

const WARNING_KEYS = {
	assumed_arch: 'arrow.inferred.warning.assumed_arch',
	emulated: 'arrow.inferred.warning.emulated',
	windows_exe_unverified: 'arrow.inferred.warning.windows_exe_unverified',
	name_mismatch: 'arrow.inferred.warning.name_mismatch',
	unpinned_rolling_tag: 'arrow.inferred.warning.unpinned_rolling_tag',
} as const satisfies Record<string, MessageKey>;

const CONFIDENCE_KEYS = {
	high: 'arrow.inferred.confidence.high',
	medium: 'arrow.inferred.confidence.medium',
	low: 'arrow.inferred.confidence.low',
} as const satisfies Record<string, MessageKey>;

function keyFor<T extends Record<string, MessageKey>>(table: T, code: string): T[keyof T] | null {
	return Object.prototype.hasOwnProperty.call(table, code) ? table[code as keyof T] : null;
}

export function warningKey(code: string): (typeof WARNING_KEYS)[keyof typeof WARNING_KEYS] | null {
	return keyFor(WARNING_KEYS, code);
}

export function confidenceKey(confidence: string): (typeof CONFIDENCE_KEYS)[keyof typeof CONFIDENCE_KEYS] | null {
	return keyFor(CONFIDENCE_KEYS, confidence);
}
