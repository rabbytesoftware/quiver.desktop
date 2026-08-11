import { useState } from 'react';

import { invoke } from '@tauri-apps/api/core';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

import { LOCAL_CONNECTION_ID } from '@/domain/connection';
import { useConnectionStore } from '@/lib/connection';
import { useTranslation } from '@/lib/i18n';
import { useMockStore } from '@/lib/mock/store';

import { Section, SettingRow } from '../section';
import { VersionUnlock } from '../version-unlock';

export function ConnectionsSettings() {
	const { t } = useTranslation();
	const connections = useConnectionStore((s) => s.connections);
	const activeId = useConnectionStore((s) => s.activeId);
	const mockEnabled = useMockStore((s) => s.enabled);

	const [name, setName] = useState('');
	const [url, setUrl] = useState('');
	const [token, setToken] = useState('');
	const [error, setError] = useState<string | null>(null);
	const [busy, setBusy] = useState(false);

	function addConnection() {
		// react-doctor-disable-next-line react-doctor/no-impure-state-updater -- `run` is a local async helper, not a setState updater
		void run(async () => {
			await invoke('add_connection', { name, url, token });
			setName('');
			setUrl('');
			setToken('');
		});
	}

	async function run(action: () => Promise<unknown>) {
		setBusy(true);
		setError(null);
		try {
			await action();
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		} finally {
			setBusy(false);
		}
	}

	return (
		<div>
			<Section
				title={t('settings.connections.hosts.title')}
				description={
					mockEnabled ? t('settings.connections.hosts.mocked') : t('settings.connections.hosts.description')
				}
			>
				{connections.map((connection) => (
					<SettingRow
						key={connection.id}
						label={connection.name}
						description={
							mockEnabled
								? t('settings.connections.host.fabricated')
								: (connection.url ??
									(connection.kind === 'local' ? t('settings.connections.host.bundled') : undefined))
						}
					>
						{connection.id === activeId ? (
							<span className="bg-primary px-2 py-0.5 text-[11px] uppercase tracking-wide text-primary-foreground">
								{t('settings.connections.host.active')}
							</span>
						) : (
							<Button
								size="sm"
								disabled={busy || mockEnabled}
								onClick={() => run(() => invoke('switch_connection', { id: connection.id }))}
							>
								{t('settings.connections.host.switch')}
							</Button>
						)}
						{connection.id !== LOCAL_CONNECTION_ID && (
							<Button
								size="sm"
								disabled={busy || mockEnabled}
								onClick={() => run(() => invoke('remove_connection', { id: connection.id }))}
							>
								{t('settings.connections.host.remove')}
							</Button>
						)}
					</SettingRow>
				))}
			</Section>

			<Section
				title={t('settings.connections.add.title')}
				description={t('settings.connections.add.description')}
			>
				<SettingRow label={t('settings.connections.add.name')}>
					<Input
						value={name}
						onChange={(e) => setName(e.target.value)}
						placeholder={t('settings.connections.add.namePlaceholder')}
						aria-label={t('settings.connections.add.nameLabel')}
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label={t('settings.connections.add.url')}>
					<Input
						value={url}
						onChange={(e) => setUrl(e.target.value)}
						type="url"
						placeholder="https://quiver.example.com"
						aria-label={t('settings.connections.add.urlLabel')}
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label={t('settings.connections.add.token')}>
					<Input
						value={token}
						onChange={(e) => setToken(e.target.value)}
						type="password"
						aria-label={t('settings.connections.add.tokenLabel')}
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label="" className="justify-end">
					<Button variant="default" disabled={busy || mockEnabled || !name || !url} onClick={addConnection}>
						{t('settings.connections.add.submit')}
					</Button>
				</SettingRow>
				{error && <p className="px-1 pt-1 text-xs text-muted-foreground">{error}</p>}
			</Section>

			<VersionUnlock />
		</div>
	);
}
