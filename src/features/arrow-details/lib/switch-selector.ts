import type { ArrowState } from '@/domain/arrow';

/** Each step resolves once it has fully happened -- `uninstall` only once the row is back to `absent`. */
export interface SwitchSteps {
	uninstall(namespace: string): Promise<void>;
	remove(namespace: string): Promise<void>;
	register(namespace: string): Promise<void>;
	install(namespace: string): Promise<void>;
}

export interface SwitchPlan {
	from: string;
	to: string;
	/** Whether `from` has anything on disk -- only then is there something to uninstall, and something to put back. */
	installed: boolean;
}

/**
 * Moves what a library entry follows. A catalog identity's selector never
 * changes, so there is no in-place switch: the old row is uninstalled and
 * forgotten, and the new identity registered (and installed, when the old one
 * was). Strictly sequential, stopping at the first failure.
 */
export async function switchSelector(plan: SwitchPlan, steps: SwitchSteps): Promise<void> {
	if (plan.installed) await steps.uninstall(plan.from);
	await steps.remove(plan.from);
	await steps.register(plan.to);
	if (plan.installed) await steps.install(plan.to);
}

const SWITCHABLE: readonly ArrowState[] = ['absent', 'ready', 'outdated'];

/** Only a row at rest can be uninstalled and forgotten: anything in flight, running or detached has to settle first. */
export function canSwitch(state: ArrowState): boolean {
	return SWITCHABLE.includes(state);
}
