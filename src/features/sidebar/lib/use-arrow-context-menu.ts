import { useRef, type MouseEvent } from 'react';

import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';

import { isTauri } from '@tauri-apps/api/core';
import { Menu } from '@tauri-apps/api/menu';

import type { ArrowEntry } from '@/domain/arrow';
import type { ArrowActionKind } from '@/features/arrow-details/lib/actions';
import { resolveRealPlatform } from '@/features/arrow-details/lib/use-real-platform';
import { useArrowStore } from '@/lib/core-store';
import { openArrowRequest, removeArrowRequest } from '@/lib/core-store/mutations/arrow';
import { runtimeMethod } from '@/lib/core-store/mutations/runtime';
import { arrowDetailQueryKey, arrowDetailQueryKeyPrefix, fetchArrowDetail } from '@/lib/core-store/queries/arrow';
import { t } from '@/lib/i18n';

import { arrowMenuItems } from './arrow-menu';

/**
 * The last menu shown. A menu's native handle must be released, but not while
 * its click is still travelling from the OS back to the item's `action` -- so a
 * menu is closed when the next one replaces it, never straight after popup.
 */
let lastMenu: Menu | null = null;

/**
 * Right-click on a sidebar arrow row: a native context menu offering what the
 * arrow's details page offers, without opening the page. Anything that needs
 * input (variables) is routed to the page instead of guessed at.
 */
export function useArrowContextMenu(arrow: ArrowEntry): (event: MouseEvent) => void {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const refreshCatalog = useArrowStore((state) => state.refreshCatalog);

	// popup() resolves only when the native menu closes; a second right-click
	// meanwhile would stack a second menu on the first.
	const busy = useRef(false);

	const showDetails = () => navigate({ to: '/arrow/$', params: { _splat: arrow.namespace } });

	async function run(kind: ArrowActionKind): Promise<void> {
		const { namespace } = arrow;
		switch (kind) {
			case 'open':
				return openArrowRequest(namespace);
			case 'execute':
			case 'stop':
			case 'install':
			case 'reinstall':
			case 'update':
			case 'uninstall': {
				await runtimeMethod({ namespace, method: kind === 'reinstall' ? 'install' : kind });
				if (kind === 'update' || kind === 'uninstall') refreshCatalog();
				return;
			}
			case 'removeFromLibrary':
				await removeArrowRequest(namespace);
				await queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix });
				return refreshCatalog();
			default:
				return showDetails();
		}
	}

	return (event) => {
		// Outside Tauri (a plain browser, tests) there is no native menu to show:
		// leave the browser's own menu alone.
		if (!isTauri()) return;
		event.preventDefault();
		if (busy.current) return;
		busy.current = true;

		void (async () => {
			try {
				const [detail, platform] = await Promise.all([
					queryClient.fetchQuery({
						queryKey: arrowDetailQueryKey(arrow.namespace),
						queryFn: () => fetchArrowDetail(arrow.namespace),
						staleTime: 5_000,
					}),
					resolveRealPlatform(),
				]);

				const actions = arrowMenuItems(detail, platform).map((item) => ({
					id: item.kind,
					text: item.viaDetails ? `${t(item.labelKey)}…` : t(item.labelKey),
					action: () => {
						if (item.viaDetails) return void showDetails();
						// The app has no toast or banner for a failure raised away from
						// the details page, so a failed action hands over to it: that
						// page shows the arrow's live state and can retry with its own
						// error banner.
						run(item.kind).catch((err) => {
							console.error(`arrow menu: ${item.kind} failed`, err);
							void showDetails();
						});
					},
				}));

				const menu = await Menu.new({
					items: [
						{ id: 'details', text: t('arrow.menu.details'), action: () => void showDetails() },
						{ item: 'Separator' },
						...actions,
					],
				});
				const previous = lastMenu;
				lastMenu = menu;
				await previous?.close().catch(() => {});
				await menu.popup();
			} catch (err) {
				console.error('arrow menu failed', err);
			} finally {
				busy.current = false;
			}
		})();
	};
}
