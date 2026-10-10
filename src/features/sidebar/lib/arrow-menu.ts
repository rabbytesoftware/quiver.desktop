import type { ArrowDetail } from '@/domain/arrow';
import { computeActions, type ArrowActionKind, type ArrowActionLabelKey } from '@/features/arrow-details/lib/actions';
import { isSelfArrow } from '@/features/arrow-details/lib/release-variables';

/** Actions the sidebar can run straight from the menu; the rest need the details page. */
const DIRECT: ReadonlySet<ArrowActionKind> = new Set([
	'open',
	'execute',
	'stop',
	'install',
	'reinstall',
	'update',
	'uninstall',
	'removeFromLibrary',
]);

const RELEASE_ACTIONS: ReadonlySet<ArrowActionKind> = new Set(['install', 'reinstall', 'update']);

export interface ArrowMenuItem {
	kind: ArrowActionKind;
	labelKey: ArrowActionLabelKey;
	/** Needs input or sequencing the details page owns: the menu item navigates there instead of running. */
	viaDetails: boolean;
}

/**
 * The menu mirrors the details page's own action set (`computeActions`), so
 * the two can never disagree about what an arrow can do right now. An action
 * that asks for variables, or is Quiver's own self-update, is routed to the
 * details page rather than guessed at.
 */
export function arrowMenuItems(detail: ArrowDetail, platform: string): ArrowMenuItem[] {
	return computeActions(detail, platform).flatMap((action) =>
		action.forceDisabled
			? []
			: [
					{
						kind: action.kind,
						labelKey: action.labelKey,
						viaDetails:
							!DIRECT.has(action.kind) ||
							action.usesVariables.length > 0 ||
							(RELEASE_ACTIONS.has(action.kind) && isSelfArrow(detail.namespace)),
					},
				]
	);
}
