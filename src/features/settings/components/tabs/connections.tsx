import { useState } from 'react';

import { invoke } from '@tauri-apps/api/core';

import { Button } from '@/components/ui/button';
import { TextInput } from '@/components/ui/controls';

import { LOCAL_CONNECTION_ID } from '@/domain/connection';
import { useConnectionStore } from '@/lib/connection';
import { useMockStore } from '@/lib/mock/store';

import { Section, SettingRow } from '../section';
import { VersionUnlock } from '../version-unlock';

/**
 * Calls `invoke` directly rather than through the `Backend` seam: these are
 * shell operations against the keyring and the Tauri store, meaningless against
 * a fabricated daemon. Under the mock they are disabled, and the panel says why.
 */
export function ConnectionsSettings() {
	const connections = useConnectionStore((s) => s.connections);
	const activeId = useConnectionStore((s) => s.activeId);
	const mockEnabled = useMockStore((s) => s.enabled);

	const [name, setName] = useState('');
	const [url, setUrl] = useState('');
	const [token, setToken] = useState('');
	const [error, setError] = useState<string | null>(null);
	const [busy, setBusy] = useState(false);

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
				title="Hosts"
				description={
					mockEnabled
						? 'The mock server is on, so this list is fabricated and cannot be changed. Turn it off in Developer to manage real hosts.'
						: 'Every quiver.core daemon this app can talk to. Switching keeps each host’s cached library separate.'
				}
			>
				{connections.map((connection) => (
					<SettingRow
						key={connection.id}
						label={connection.name}
						description={
							mockEnabled
								? 'Fabricated — there is no daemon behind this'
								: (connection.url ?? (connection.kind === 'local' ? 'Bundled daemon' : undefined))
						}
					>
						{connection.id === activeId ? (
							<span className="bg-fill px-2 py-0.5 text-[11px] uppercase tracking-wide text-fill-ink">
								Active
							</span>
						) : (
							<Button
								disabled={busy || mockEnabled}
								onClick={() => run(() => invoke('switch_connection', { id: connection.id }))}
							>
								Switch
							</Button>
						)}
						{/* `manager.rs` refuses to remove the local connection. */}
						{connection.id !== LOCAL_CONNECTION_ID && (
							<Button
								disabled={busy || mockEnabled}
								onClick={() => run(() => invoke('remove_connection', { id: connection.id }))}
							>
								Remove
							</Button>
						)}
					</SettingRow>
				))}
			</Section>

			<Section
				title="Add a host"
				description="A remote quiver.core daemon. The token is stored in the OS keychain, never in the app’s own storage."
			>
				<SettingRow label="Name">
					<TextInput
						value={name}
						onChange={setName}
						placeholder="Basement box"
						aria-label="Host name"
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label="URL">
					<TextInput
						value={url}
						onChange={setUrl}
						type="url"
						placeholder="https://quiver.example.com"
						aria-label="Host URL"
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label="Token">
					<TextInput
						value={token}
						onChange={setToken}
						type="password"
						aria-label="Host token"
						className="w-[200px]"
					/>
				</SettingRow>
				<SettingRow label="" className="justify-end">
					<Button
						variant="default"
						disabled={busy || mockEnabled || !name || !url}
						onClick={() =>
							run(async () => {
								await invoke('add_connection', { name, url, token });
								setName('');
								setUrl('');
								setToken('');
							})
						}
					>
						Add host
					</Button>
				</SettingRow>
				{error && <p className="px-1 pt-1 text-[12px] text-ink-2">{error}</p>}
			</Section>

			<VersionUnlock />
		</div>
	);
}
