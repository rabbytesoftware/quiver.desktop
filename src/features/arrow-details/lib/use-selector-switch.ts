import { useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowDetail, ArrowState } from '@/domain/arrow';
import { installTagFor, isSelfArrow, releaseVariables } from '@/features/arrow-details/lib/release-variables';
import { type SwitchError, switchSelector } from '@/features/arrow-details/lib/switch-selector';
import { type SwitchFailure, useSelectorSwitchStore } from '@/features/arrow-details/stores/selector-switch-store';
import { useArrowStore, useInstall, useRegisterArrow, useRemoveArrow, useUninstall } from '@/lib/core-store';
import type { ArrowDetailDTO } from '@/lib/core-store/dtos/v0/arrow';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';
import { namespaceSegment, withSelector } from '@/lib/namespace';
import { apiFetch } from '@/lib/transport/api';

/** Long enough for any real uninstall; only a row that never settles reaches it. */
const UNINSTALL_TIMEOUT_MS = 10 * 60 * 1000;

const NOT_INSTALLED: readonly ArrowState[] = ['absent', 'removed'];

export interface SelectorSwitch {
	/** True while a switch from or to this identity is in flight. */
	pending: boolean;
	/** Registering the new identity failed, so nothing was changed. */
	registerError: string | null;
	/** A later step failed -- the new identity exists; see `SwitchFailure` for what else happened. */
	failure: SwitchFailure | null;
	dismissRegisterError(): void;
	dismissFailure(): void;
	/** Registers `selector` beside this row, then uninstalls and forgets this row and installs the new one when this one was installed. */
	switchTo(selector: string): Promise<void>;
}

/**
 * Settles once the live store reports `namespace` not installed: resolves at
 * `absent`/`removed` (including when `check()` finds it there already, e.g.
 * after an uninstall core answered as a no-op), and rejects when the row
 * settles anywhere else after having started uninstalling, or when
 * `timeoutMs` passes first. Subscribed BEFORE the uninstall is requested, so
 * a fast transition cannot slip past it.
 */
export function untilUninstalled(
	namespace: string,
	timeoutMs: number = UNINSTALL_TIMEOUT_MS
): { settled: Promise<void>; check: () => void; cancel: () => void } {
	let check = (): void => {};
	let cancel = (): void => {};
	const settled = new Promise<void>((resolve, reject) => {
		let started = false;
		let unsubscribe = (): void => {};
		const finish = (err?: Error): void => {
			unsubscribe();
			clearTimeout(timer);
			if (err) reject(err);
			else resolve();
		};
		const inspect = (state: ArrowState | undefined): void => {
			if (state === undefined) return;
			if (NOT_INSTALLED.includes(state)) return finish();
			if (state === 'uninstalling') {
				started = true;
				return;
			}
			if (started) finish(new Error(`${namespace} did not finish uninstalling (it is ${state})`));
		};
		const timer = setTimeout(
			() => finish(new Error(`${namespace} was not uninstalled within ${Math.round(timeoutMs / 1000)}s`)),
			timeoutMs
		);
		unsubscribe = useArrowStore.subscribe((store) => inspect(store.arrows.get(namespace)?.state));
		check = () => inspect(useArrowStore.getState().arrows.get(namespace)?.state);
		cancel = () => {
			unsubscribe();
			clearTimeout(timer);
		};
	});
	return { settled, check, cancel };
}

/**
 * Quiver's own row installs a release asset nothing in its manifest can name.
 * The new row's resolved ref is only known once core has registered it, so
 * its detail is read back first -- see `installTagFor`.
 */
async function installVariables(namespace: string): Promise<Record<string, string>> {
	if (!isSelfArrow(namespace)) return {};
	const detail = await apiFetch<ArrowDetailDTO>(`/v0/arrow/${namespaceSegment(namespace)}`);
	return releaseVariables(installTagFor({ resolved_ref: detail.resolved_ref ?? '' }));
}

/**
 * The "switch = register + uninstall + reinstall" flow behind the Hero's
 * switch dialog. `onRegistered` receives the new identity the moment it
 * exists, before the old row is touched, so the page never sits on an
 * identity that is about to be forgotten. A failure at register leaves
 * everything as it was (`registerError`); a later one leaves the new identity
 * registered and is reported through the switch store (`failure`), which
 * outlives the page it started on.
 */
export function useSelectorSwitch(
	detail: ArrowDetail,
	values: Record<string, string>,
	onRegistered: ((namespace: string) => void) | undefined
): SelectorSwitch {
	const queryClient = useQueryClient();
	const uninstall = useUninstall();
	const remove = useRemoveArrow();
	const register = useRegisterArrow();
	const install = useInstall();
	const [registerError, setRegisterError] = useState<string | null>(null);
	const active = useSelectorSwitchStore((state) => state.active);
	const failure = useSelectorSwitchStore((state) => state.failure);

	async function uninstallAndWait(namespace: string): Promise<void> {
		const { settled, check, cancel } = untilUninstalled(namespace);
		try {
			await uninstall.mutateAsync({ namespace });
		} catch (err) {
			cancel();
			throw err;
		}
		check();
		await settled;
	}

	async function switchTo(selector: string): Promise<void> {
		const from = detail.namespace;
		const to = withSelector(from, selector);
		if (to === from) return;
		const store = useSelectorSwitchStore.getState();
		setRegisterError(null);
		store.begin(from, to);
		try {
			await switchSelector(
				{ from, to, installed: detail.state !== 'absent' },
				{
					register: (namespace) => register.mutateAsync({ namespace }),
					registered: (namespace) => {
						useArrowStore.getState().refreshCatalog();
						onRegistered?.(namespace);
					},
					uninstall: uninstallAndWait,
					remove: (namespace) => remove.mutateAsync({ namespace }),
					install: async (namespace) =>
						install.mutateAsync({
							namespace,
							variables: { ...values, ...(await installVariables(namespace)) },
						}),
				}
			);
			store.end();
		} catch (err) {
			const { step, reason } = err as SwitchError;
			if (step === 'register') {
				setRegisterError(reason);
				store.end();
			} else {
				store.end({ step, from, to, reason });
			}
		} finally {
			useArrowStore.getState().refreshCatalog();
			await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
		}
	}

	return {
		pending: active !== null && (active.from === detail.namespace || active.to === detail.namespace),
		registerError,
		failure,
		dismissRegisterError: () => setRegisterError(null),
		dismissFailure: () => useSelectorSwitchStore.getState().dismiss(),
		switchTo,
	};
}
