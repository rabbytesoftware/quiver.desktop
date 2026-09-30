import { useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowDetail } from '@/domain/arrow';
import { isSelfArrow, releaseVariables } from '@/features/arrow-details/lib/release-variables';
import { switchSelector } from '@/features/arrow-details/lib/switch-selector';
import { useArrowStore, useInstall, useRegisterArrow, useRemoveArrow, useUninstall } from '@/lib/core-store';
import type { ArrowDetailDTO } from '@/lib/core-store/dtos/v0/arrow';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';
import { namespaceSegment, withSelector } from '@/lib/namespace';
import { apiFetch } from '@/lib/transport/api';

export interface SelectorSwitch {
	/** True from the first call until the new identity is registered (and installed) or a step failed. */
	pending: boolean;
	/** The failing step's reason; the old row is left as it was at that point. */
	error: string | null;
	dismissError(): void;
	/** Uninstalls and forgets this row, then registers (and installs, when this one was) `selector` in its place. */
	switchTo(selector: string): Promise<void>;
}

/**
 * Resolves once the live store reports `namespace` back at `absent`, and
 * rejects when it settles anywhere else after having started uninstalling.
 * Subscribed BEFORE the uninstall is requested, so a fast transition cannot
 * slip past it.
 */
function untilUninstalled(namespace: string): { settled: Promise<void>; cancel: () => void } {
	let cancel = (): void => {};
	const settled = new Promise<void>((resolve, reject) => {
		let started = false;
		const unsubscribe = useArrowStore.subscribe((state) => {
			const entry = state.arrows.get(namespace);
			if (!entry) return;
			if (entry.state === 'uninstalling') {
				started = true;
				return;
			}
			if (entry.state === 'absent') {
				unsubscribe();
				resolve();
				return;
			}
			if (started) {
				unsubscribe();
				reject(new Error(`${namespace} did not finish uninstalling (it is ${entry.state})`));
			}
		});
		cancel = unsubscribe;
	});
	return { settled, cancel };
}

/**
 * Quiver's own row installs a release asset nothing in its manifest can name.
 * The new row's target is only known once core has registered it, so its
 * detail is read back first -- see `releaseTagFor`.
 */
async function installVariables(namespace: string): Promise<Record<string, string>> {
	if (!isSelfArrow(namespace)) return {};
	const detail = await apiFetch<ArrowDetailDTO>(`/v0/arrow/${namespaceSegment(namespace)}`);
	return releaseVariables(detail.available?.ref || detail.resolved_ref || undefined);
}

/**
 * The "switch = uninstall + reinstall" flow behind the Hero's switch dialog.
 * `onIdentityChange` receives the new identity once it exists, since the page
 * the switch started from names a row that is gone by then.
 */
export function useSelectorSwitch(
	detail: ArrowDetail,
	values: Record<string, string>,
	onIdentityChange: ((namespace: string) => void) | undefined
): SelectorSwitch {
	const queryClient = useQueryClient();
	const uninstall = useUninstall();
	const remove = useRemoveArrow();
	const register = useRegisterArrow();
	const install = useInstall();
	const [pending, setPending] = useState(false);
	const [error, setError] = useState<string | null>(null);

	async function uninstallAndWait(namespace: string): Promise<void> {
		const { settled, cancel } = untilUninstalled(namespace);
		try {
			await uninstall.mutateAsync({ namespace });
		} catch (err) {
			cancel();
			throw err;
		}
		await settled;
	}

	async function switchTo(selector: string): Promise<void> {
		const to = withSelector(detail.namespace, selector);
		if (to === detail.namespace) return;
		setPending(true);
		setError(null);
		try {
			await switchSelector(
				{ from: detail.namespace, to, installed: detail.state !== 'absent' },
				{
					uninstall: uninstallAndWait,
					remove: (namespace) => remove.mutateAsync({ namespace }),
					register: (namespace) => register.mutateAsync({ namespace }),
					install: async (namespace) =>
						install.mutateAsync({
							namespace,
							variables: { ...values, ...(await installVariables(namespace)) },
						}),
				}
			);
			await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
			onIdentityChange?.(to);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		} finally {
			setPending(false);
		}
	}

	return { pending, error, dismissError: () => setError(null), switchTo };
}
