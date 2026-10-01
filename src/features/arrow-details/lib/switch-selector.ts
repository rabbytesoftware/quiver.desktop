import type { ArrowState } from '@/domain/arrow';

export type SwitchStep = 'register' | 'uninstall' | 'remove' | 'install';

/** Each step resolves once it has fully happened -- `uninstall` only once the row is no longer installed. */
export interface SwitchSteps {
	register(namespace: string): Promise<void>;
	uninstall(namespace: string): Promise<void>;
	remove(namespace: string): Promise<void>;
	install(namespace: string): Promise<void>;
	/** Called once the new identity exists, before anything of the old row is touched. */
	registered(namespace: string): void;
}

export interface SwitchPlan {
	from: string;
	to: string;
	/** Whether `from` has anything on disk -- only then is there something to uninstall, and something to put back. */
	installed: boolean;
}

/** Which step of a switch failed, and why. Every step before it has happened; none after it has. */
export class SwitchError extends Error {
	readonly step: SwitchStep;
	readonly reason: string;

	constructor(step: SwitchStep, cause: unknown) {
		const reason = cause instanceof Error ? cause.message : String(cause);
		super(`${step}: ${reason}`);
		this.name = 'SwitchError';
		this.step = step;
		this.reason = reason;
	}
}

async function run(step: SwitchStep, action: () => Promise<void>): Promise<void> {
	try {
		await action();
	} catch (err) {
		throw new SwitchError(step, err);
	}
}

/**
 * Moves what a library entry follows. A catalog identity's selector never
 * changes, so there is no in-place switch. The new identity is registered
 * FIRST: rows coexist, so registering changes nothing about the old one, and a
 * selector core cannot resolve fails there with nothing lost. Only then is the
 * old row uninstalled (when it was installed) and forgotten, and the new one
 * installed in its place. Strictly sequential; a failure throws a
 * `SwitchError` naming the step it stopped at.
 */
export async function switchSelector(plan: SwitchPlan, steps: SwitchSteps): Promise<void> {
	await run('register', () => steps.register(plan.to));
	steps.registered(plan.to);
	if (plan.installed) await run('uninstall', () => steps.uninstall(plan.from));
	await run('remove', () => steps.remove(plan.from));
	if (plan.installed) await run('install', () => steps.install(plan.to));
}

const SWITCHABLE: readonly ArrowState[] = ['absent', 'ready', 'outdated'];

/** Only a row at rest can be uninstalled and forgotten: anything in flight, running or detached has to settle first. */
export function canSwitch(state: ArrowState): boolean {
	return SWITCHABLE.includes(state);
}
