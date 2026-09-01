import type { ArrowDetail } from '@/domain/arrow';
import type { MessageKey } from '@/lib/i18n';

import { ArrowUpIcon, CheckIcon, CircleIcon, PackageIcon, RadioIcon, TriangleAlertIcon } from 'lucide-react';

/** Drives which status-badge treatment the hero shows -- icon shape and color family, not literal colors. */
export type ArrowIconKind = 'idle' | 'ready' | 'busy' | 'active' | 'up' | 'problem' | 'archived';

/** Shared with any other spot that shows a compact status badge for an arrow (the hero, a dependency row). */
export const STATUS_ICONS: Record<ArrowIconKind, typeof CircleIcon> = {
	idle: CircleIcon,
	ready: CheckIcon,
	busy: CircleIcon, // callers override this with a spinner
	active: RadioIcon,
	up: ArrowUpIcon,
	problem: TriangleAlertIcon,
	archived: PackageIcon,
};

export const STATUS_BADGE_VARIANT: Record<ArrowIconKind, 'default' | 'outline' | 'secondary' | 'error'> = {
	idle: 'outline',
	ready: 'outline',
	busy: 'secondary',
	active: 'default',
	up: 'outline',
	problem: 'error',
	archived: 'outline',
};

export interface ArrowStatus {
	labelKey: Extract<MessageKey, `arrow.state.${string}`>;
	iconKind: ArrowIconKind;
}

/**
 * "Discovered" (browsed, never added to the library) is not a real
 * `ArrowState` value -- it's simply `user_installed === false`. Every other
 * state maps 1:1 to the badge treatment.
 */
export function computeStatus(detail: Pick<ArrowDetail, 'state' | 'user_installed'>): ArrowStatus {
	if (!detail.user_installed) {
		return { labelKey: 'arrow.state.discovered', iconKind: 'idle' };
	}

	switch (detail.state) {
		case 'absent':
			return { labelKey: 'arrow.state.absent', iconKind: 'idle' };
		case 'installing':
			return { labelKey: 'arrow.state.installing', iconKind: 'busy' };
		case 'ready':
			return { labelKey: 'arrow.state.ready', iconKind: 'ready' };
		case 'outdated':
			return { labelKey: 'arrow.state.outdated', iconKind: 'up' };
		case 'updating':
			return { labelKey: 'arrow.state.updating', iconKind: 'busy' };
		case 'running':
			return { labelKey: 'arrow.state.running', iconKind: 'active' };
		case 'stopping':
			return { labelKey: 'arrow.state.stopping', iconKind: 'busy' };
		case 'draining':
			return { labelKey: 'arrow.state.draining', iconKind: 'busy' };
		case 'detached':
			return { labelKey: 'arrow.state.detached', iconKind: 'problem' };
		case 'uninstalling':
			return { labelKey: 'arrow.state.uninstalling', iconKind: 'busy' };
		case 'removed':
			return { labelKey: 'arrow.state.removed', iconKind: 'archived' };
		default:
			return { labelKey: 'arrow.state.absent', iconKind: 'idle' };
	}
}

export type ArrowProblem =
	| { reason: 'detached' }
	| {
			reason: 'failed';
			/** The failed step's own error text, when core reported one -- the caller shows this verbatim (it's technical wire text, not a translatable sentence), or falls back to a generic message when absent. */
			detail?: string;
	  };

/**
 * Whether a "problem" chip belongs in the tags row -- a failed run or a
 * detached process. The caller resolves `reason: 'detached'` to
 * `t('arrow.problem.detachedNote')`; for `'failed'`, `detail` (when present)
 * is the specific failed step's own error text, straight from core, not
 * translated content. Returns `null` when there's nothing to flag.
 */
export function problemMessage(detail: Pick<ArrowDetail, 'state' | 'last_return'>): ArrowProblem | null {
	if (detail.state === 'detached') {
		return { reason: 'detached' };
	}
	if (detail.last_return?.outcome === 'failed') {
		const failedStep = detail.last_return.steps.find((step) => step.status === 'failed');
		return { reason: 'failed', detail: failedStep?.error };
	}
	return null;
}
