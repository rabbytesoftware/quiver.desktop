import { useEffect, useRef, type JSX, type KeyboardEvent } from 'react';

import { Dialog, DialogPopup } from '@/components/ui/dialog';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { GlobeIcon, MonitorIcon, SearchIcon } from 'lucide-react';

import { useConnectionActions } from '../hooks/use-connection-actions';
import { clampIndex, filterConnections } from '../lib/command-filter';
import { useRemoteStore } from '../stores/remote-store';

const KBD =
	'inline-flex h-4.5 min-w-4.5 items-center justify-center rounded border border-border px-1 font-mono text-[10px] text-muted-foreground';

export function CommandPalette(): JSX.Element {
	const { t } = useTranslation();
	const connections = useConnectionStore((s) => s.connections);
	const activeId = useConnectionStore((s) => s.activeId);
	const status = useStatusStore((s) => s.status);

	const cmdOpen = useRemoteStore((s) => s.cmdOpen);
	const cmdQuery = useRemoteStore((s) => s.cmdQuery);
	const cmdIndex = useRemoteStore((s) => s.cmdIndex);
	const closeCmd = useRemoteStore((s) => s.closeCmd);
	const setCmdQuery = useRemoteStore((s) => s.setCmdQuery);
	const setCmdIndex = useRemoteStore((s) => s.setCmdIndex);

	const { connect } = useConnectionActions();
	const inputRef = useRef<HTMLInputElement>(null);

	useEffect(() => {
		if (cmdOpen) inputRef.current?.focus();
	}, [cmdOpen]);

	const filtered = filterConnections(connections, cmdQuery);

	function select(id: string, name: string) {
		closeCmd();
		if (id === activeId && status !== 'disconnected') return;
		void connect(id, name);
	}

	function onInputKeyDown(event: KeyboardEvent<HTMLInputElement>) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			setCmdIndex(clampIndex(cmdIndex + 1, filtered.length));
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			setCmdIndex(clampIndex(cmdIndex - 1, filtered.length));
		} else if (event.key === 'Enter') {
			event.preventDefault();
			const item = filtered[cmdIndex];
			if (item) select(item.id, item.name);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			closeCmd();
		}
	}

	return (
		<Dialog onOpenChange={(open) => !open && closeCmd()} open={cmdOpen}>
			<DialogPopup className="top-[18%] max-w-[480px] translate-y-0" showCloseButton={false}>
				<div className="flex items-center gap-2.5 border-b border-border px-4 py-3">
					<SearchIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
					<input
						aria-label={t('remote.command.groupLabel')}
						className="h-6 flex-1 border-none bg-transparent text-[14.5px] outline-none placeholder:text-muted-foreground"
						onChange={(event) => setCmdQuery(event.target.value)}
						onKeyDown={onInputKeyDown}
						placeholder={t('remote.command.placeholder')}
						ref={inputRef}
						value={cmdQuery}
					/>
					<span className={KBD}>esc</span>
				</div>

				<div className="max-h-80 overflow-y-auto p-1.5">
					{filtered.length > 0 ? (
						<>
							<div className="px-2 pt-1 pb-1.5 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
								{t('remote.command.groupLabel')}
							</div>
							{filtered.map((connection, index) => {
								const Icon = connection.kind === 'local' ? MonitorIcon : GlobeIcon;
								return (
									<button
										className={`flex w-full items-center gap-2.5 rounded-md px-2 py-2 text-left ${index === cmdIndex ? 'bg-accent' : ''}`}
										key={connection.id}
										onClick={() => select(connection.id, connection.name)}
										type="button"
									>
										<div className="flex size-6.5 shrink-0 items-center justify-center rounded-sm bg-muted text-muted-foreground">
											<Icon aria-hidden="true" className="size-3.5" />
										</div>
										<div className="min-w-0 flex-1">
											<div className="text-[13px] font-medium">{connection.name}</div>
											<div className="truncate font-mono text-[11.5px] text-muted-foreground">
												{connection.kind === 'local'
													? t('remote.local.subtitle')
													: connection.url}
											</div>
										</div>
										{connection.id === activeId && (
											<span className="shrink-0 text-[11px] text-muted-foreground">
												{t('remote.command.current')}
											</span>
										)}
									</button>
								);
							})}
						</>
					) : (
						<p className="px-2 py-7 text-center text-[12.5px] text-muted-foreground">
							{t('remote.command.empty', { query: cmdQuery })}
						</p>
					)}
				</div>

				<div className="flex items-center gap-4 border-t border-border bg-muted/55 px-4 py-2.5">
					<span className="flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
						<span className={KBD}>↑</span>
						<span className={KBD}>↓</span> {t('remote.command.navigate')}
					</span>
					<span className="flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
						<span className={KBD}>↵</span> {t('remote.command.select')}
					</span>
					<span className="flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
						<span className={KBD}>esc</span> {t('remote.command.close')}
					</span>
				</div>
			</DialogPopup>
		</Dialog>
	);
}
