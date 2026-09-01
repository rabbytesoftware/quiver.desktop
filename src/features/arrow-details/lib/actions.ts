import type { ArrowDetail, ArrowStepDefinition } from '@/domain/arrow';
import { targetForPlatform } from '@/domain/arrow';
import type { MessageKey } from '@/lib/i18n';

/**
 * What a hero action button actually invokes. `execute` is quiver.core's one
 * universal "go" action (`POST /v0/runtime/:ns/execute`, reading
 * `Target.Lifecycle.Execute` -- never a custom method name); `restart` is a
 * client-side stop-then-execute sequence, not a single core call.
 */
export type ArrowActionKind =
	| 'addToLibrary'
	| 'install'
	| 'removeFromLibrary'
	| 'execute'
	| 'uninstall'
	| 'update'
	| 'stop'
	| 'restart'
	| 'reinstall';

export type ArrowActionVariant = 'default' | 'outline' | 'destructive' | 'destructive-outline';

/**
 * A narrower alias than the full `MessageKey` union, covering only the
 * plain, parameter-free `arrow.action.*` strings this module actually uses.
 * `t()`'s signature requires a params argument whenever the *type* it's
 * called with could plausibly need one -- passing the full `MessageKey`
 * union here would force every caller to pass a pointless `{}`. Keep this in
 * sync with the literal keys used below (TypeScript itself will catch a
 * typo'd key against `en.ts`, since this is still constrained to be a
 * `MessageKey` subset).
 */
export type ArrowActionLabelKey = Extract<MessageKey, `arrow.action.${string}`>;

export interface ArrowAction {
	kind: ArrowActionKind;
	labelKey: ArrowActionLabelKey;
	/** Set only when `forceBusy` is true -- the label to show in place of `labelKey` while in flight. */
	busyLabelKey?: ArrowActionLabelKey;
	variant: ArrowActionVariant;
	/** What the info trigger previews. Empty when core doesn't expose a step list for this action (e.g. "Add to Library"). */
	steps: ArrowStepDefinition[];
	/** Variable names this call's `variables` body should carry -- core has no per-lifecycle-action variable scoping, so this is either every declared variable or none. */
	usesVariables: string[];
	/** This is the action that's currently running (active_run.method matches it) -- disabled, shows busyLabelKey and a spinner, but its info trigger stays clickable. */
	forceBusy: boolean;
	/** Not runnable from the current state -- plainly greyed, no busy label, info trigger stays clickable. */
	forceDisabled: boolean;
}

function allVariableNames(detail: ArrowDetail): string[] {
	return detail.variables.map((v) => v.name);
}

/**
 * The hero's action set for the current state, verified state-by-state
 * against quiver.core's real gating this session (see docs/arrow-details-spec.md
 * §4 and §7.2). `execute` (the universal "Start") is hard-gated to `ready`
 * only, unconditionally, for every arrow, with no manifest override possible
 * -- never render it enabled outside `ready`, even speculatively.
 */
export function computeActions(detail: ArrowDetail, platform: string): ArrowAction[] {
	if (!detail.user_installed) {
		return [
			{
				kind: 'addToLibrary',
				labelKey: 'arrow.action.addToLibrary',
				variant: 'default',
				steps: [],
				usesVariables: [],
				forceBusy: false,
				forceDisabled: false,
			},
		];
	}

	const target = targetForPlatform(detail.targets, platform);
	const lifecycle = target?.lifecycle;
	const hasExecute = (lifecycle?.execute.length ?? 0) > 0;
	const configVars = allVariableNames(detail);

	// Only ever called under an `if (hasExecute)` guard, which already implies
	// `lifecycle` and `lifecycle.execute` are both non-empty -- `lifecycle!` is
	// safe here, not a shortcut around a real null case.
	const execute = (forceDisabled: boolean, variant: ArrowActionVariant = 'outline'): ArrowAction => ({
		kind: 'execute',
		labelKey: 'arrow.action.start',
		variant,
		steps: lifecycle!.execute,
		usesVariables: configVars,
		forceBusy: false,
		forceDisabled,
	});

	switch (detail.state) {
		case 'absent':
			return [
				{
					kind: 'install',
					labelKey: 'arrow.action.install',
					variant: 'default',
					steps: lifecycle?.install ?? [],
					usesVariables: configVars,
					forceBusy: false,
					forceDisabled: false,
				},
				{
					kind: 'removeFromLibrary',
					labelKey: 'arrow.action.removeFromLibrary',
					variant: 'outline',
					steps: [],
					usesVariables: [],
					forceBusy: false,
					forceDisabled: false,
				},
			];

		case 'installing':
			return [
				{
					kind: 'install',
					labelKey: 'arrow.action.install',
					busyLabelKey: 'arrow.action.installing',
					variant: 'default',
					steps: lifecycle?.install ?? [],
					usesVariables: configVars,
					forceBusy: true,
					forceDisabled: false,
				},
				{
					kind: 'removeFromLibrary',
					labelKey: 'arrow.action.removeFromLibrary',
					variant: 'outline',
					steps: [],
					usesVariables: [],
					forceBusy: false,
					forceDisabled: true,
				},
			];

		case 'ready': {
			const actions: ArrowAction[] = [];
			if (hasExecute) actions.push(execute(false, 'default'));
			actions.push({
				kind: 'uninstall',
				labelKey: 'arrow.action.uninstall',
				variant: 'outline',
				steps: lifecycle?.uninstall ?? [],
				usesVariables: [],
				forceBusy: false,
				forceDisabled: false,
			});
			return actions;
		}

		case 'outdated': {
			const actions: ArrowAction[] = [
				{
					kind: 'update',
					labelKey: 'arrow.action.update',
					variant: 'default',
					steps: lifecycle?.update ?? [],
					usesVariables: [],
					forceBusy: false,
					forceDisabled: false,
				},
			];
			// Never conditional: execute is ready-only for every arrow, always --
			// not gated by manifest AvailableIn the way custom methods are.
			if (hasExecute) actions.push(execute(true));
			return actions;
		}

		case 'updating': {
			const actions: ArrowAction[] = [
				{
					kind: 'update',
					labelKey: 'arrow.action.update',
					busyLabelKey: 'arrow.action.updating',
					variant: 'default',
					steps: lifecycle?.update ?? [],
					usesVariables: [],
					forceBusy: true,
					forceDisabled: false,
				},
			];
			if (hasExecute) actions.push(execute(true));
			return actions;
		}

		case 'running': {
			const actions: ArrowAction[] = [
				{
					kind: 'stop',
					labelKey: 'arrow.action.stop',
					variant: 'destructive-outline',
					steps: lifecycle?.stop ?? [],
					usesVariables: [],
					forceBusy: false,
					forceDisabled: false,
				},
			];
			if (hasExecute) {
				actions.push({
					kind: 'restart',
					labelKey: 'arrow.action.restart',
					variant: 'outline',
					// hasExecute already implies lifecycle is defined.
					steps: [...lifecycle!.stop, ...lifecycle!.execute],
					usesVariables: configVars,
					forceBusy: false,
					forceDisabled: false,
				});
			}
			return actions;
		}

		case 'stopping':
		case 'draining': {
			// No escalation exists here: `BeginStop.Validate` explicitly rejects a
			// second `stop` call while one is already in flight (409), so there is
			// no "force" path to offer -- verified against core this session. This
			// mirrors installing/updating/uninstalling: the in-flight action stays
			// visible-but-busy, with no alternate action beyond a disabled sibling.
			const busyLabelKey: ArrowActionLabelKey =
				detail.state === 'stopping' ? 'arrow.action.stopping' : 'arrow.action.draining';
			const actions: ArrowAction[] = [
				{
					kind: 'stop',
					labelKey: 'arrow.action.stop',
					busyLabelKey,
					variant: 'destructive-outline',
					steps: lifecycle?.stop ?? [],
					usesVariables: [],
					forceBusy: true,
					forceDisabled: false,
				},
			];
			if (hasExecute) {
				actions.push({
					kind: 'restart',
					labelKey: 'arrow.action.restart',
					variant: 'outline',
					// hasExecute already implies lifecycle is defined.
					steps: [...lifecycle!.stop, ...lifecycle!.execute],
					usesVariables: configVars,
					forceBusy: false,
					forceDisabled: true,
				});
			}
			return actions;
		}

		case 'detached':
			// `BeginStop.Validate` explicitly allows `Detached` as a starting state
			// for the ordinary `stop` call -- verified this session. There is no
			// separate "force stop" capability in core at all (a second `stop`
			// while one is in flight is rejected, not escalated), so this is a
			// plain Stop, not a distinct action.
			return [
				{
					kind: 'stop',
					labelKey: 'arrow.action.stop',
					variant: 'destructive-outline',
					steps: lifecycle?.stop ?? [],
					usesVariables: [],
					forceBusy: false,
					forceDisabled: false,
				},
			];

		case 'uninstalling': {
			const actions: ArrowAction[] = [];
			if (hasExecute) actions.push(execute(true, 'default'));
			actions.push({
				kind: 'uninstall',
				labelKey: 'arrow.action.uninstall',
				busyLabelKey: 'arrow.action.uninstalling',
				variant: 'outline',
				steps: lifecycle?.uninstall ?? [],
				usesVariables: [],
				forceBusy: true,
				forceDisabled: false,
			});
			return actions;
		}

		case 'removed':
			return [
				{
					kind: 'reinstall',
					labelKey: 'arrow.action.reinstall',
					variant: 'outline',
					steps: lifecycle?.install ?? [],
					usesVariables: configVars,
					forceBusy: false,
					forceDisabled: false,
				},
			];

		default:
			return [];
	}
}
