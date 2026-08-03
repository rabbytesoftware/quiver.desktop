import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { NumberField, Select, Slider, Switch } from '@/components/ui/controls';

import { mockForcedByEnv } from '@/lib/mock/preference';
import { FAULT_KEYS, FAULT_LABELS, useMockStore } from '@/lib/mock/store';
import { SCENARIOS } from '@/lib/mock/world/scenarios';
import type { ScenarioName } from '@/lib/mock/world/types';

import { Section, SettingRow } from '../section';

export function DeveloperSettings() {
	const enabled = useMockStore((s) => s.enabled);
	const scenario = useMockStore((s) => s.scenario);
	const latency = useMockStore((s) => s.latency);
	const errorRate = useMockStore((s) => s.errorRate);
	const unreachable = useMockStore((s) => s.unreachable);
	const faults = useMockStore((s) => s.faults);

	const setLatency = useMockStore((s) => s.setLatency);
	const setErrorRate = useMockStore((s) => s.setErrorRate);
	const setUnreachable = useMockStore((s) => s.setUnreachable);
	const setFault = useMockStore((s) => s.setFault);
	const resetFaults = useMockStore((s) => s.resetFaults);
	const resetChaos = useMockStore((s) => s.resetChaos);
	const applyAndReload = useMockStore((s) => s.applyAndReload);

	const forcedByEnv = mockForcedByEnv();

	// Local until Apply, so browsing options does not reload the app.
	const [pending, setPending] = useState<ScenarioName>(scenario);
	const scenarioChanged = pending !== scenario;
	const anyFault = FAULT_KEYS.some((k) => faults[k] > 0);

	return (
		<div>
			<Section
				title="Mock server"
				description="Replaces the quiver.core daemon with an in-memory one. Nothing is contacted over the network, and your real library is untouched — mock data lives in its own cache partition."
			>
				<SettingRow
					label="Use the mock server"
					description={
						forcedByEnv
							? 'Forced on by VITE_QUIVER_MOCK for this run — started by `make dev-mock` or `make dev-web`. Restart without it to get the switch back.'
							: 'Turning this on or off reloads the app: which backend is in use is decided once at startup.'
					}
				>
					<Switch
						checked={enabled || forcedByEnv}
						// Left live it would write `enabled: false`, reload, and come
						// straight back on because the environment still says so.
						disabled={forcedByEnv}
						onCheckedChange={(next) => applyAndReload({ enabled: next })}
						aria-label="Use the mock server"
					/>
				</SettingRow>

				<SettingRow label="Scenario" description={SCENARIOS.find((s) => s.name === pending)?.summary}>
					<Select
						value={pending}
						onValueChange={setPending}
						options={SCENARIOS.map((s) => ({ value: s.name, label: s.label }))}
						className="w-[132px]"
						aria-label="Mock scenario"
					/>
					<Button
						variant="default"
						disabled={!scenarioChanged}
						onClick={() => applyAndReload({ enabled: true, scenario: pending })}
					>
						Apply
					</Button>
				</SettingRow>
			</Section>

			{/* Shown rather than hidden when the mock is off, so the tab does not
			    change shape depending on a switch three rows up. */}
			<Section
				title="Chaos"
				description={
					enabled
						? 'Applies to the next request. Nothing here is persisted — it all resets when the app restarts.'
						: 'Inert while the mock server is off. quiver.core has no equivalent of these, so they cannot be applied to a real daemon.'
				}
			>
				<SettingRow label="Latency" description="Delay added to every mock response.">
					<NumberField
						value={latency}
						onValueChange={setLatency}
						min={0}
						max={10000}
						step={50}
						suffix="ms"
						aria-label="Latency in milliseconds"
					/>
				</SettingRow>

				<SettingRow label="Error rate" description="Chance each request comes back as a daemon-side 500.">
					<NumberField
						value={errorRate}
						onValueChange={setErrorRate}
						min={0}
						max={100}
						step={5}
						suffix="%"
						aria-label="Error rate percentage"
					/>
				</SettingRow>

				<SettingRow
					label="Daemon unreachable"
					description="Answers every request the way the Rust proxy answers a refused socket — a 502 carrying x-quiver-proxy. This is the only fault that exercises the retry ladder and reaches the Disconnected screen."
				>
					<Switch checked={unreachable} onCheckedChange={setUnreachable} aria-label="Daemon unreachable" />
				</SettingRow>

				<SettingRow label="" className="justify-end">
					<Button
						variant="outline"
						onClick={resetChaos}
						disabled={latency === 0 && errorRate === 0 && !unreachable}
					>
						Reset chaos
					</Button>
				</SettingRow>
			</Section>

			<Section
				title="Fault injection"
				description="Force one route family to fail, so you can see a single panel's error state without breaking the rest of the app."
			>
				{FAULT_KEYS.map((key) => (
					<SettingRow key={key} label={FAULT_LABELS[key]}>
						<Slider
							value={faults[key]}
							onValueChange={(v) => setFault(key, v)}
							step={5}
							className="w-[140px]"
							aria-label={`${FAULT_LABELS[key]} fault rate`}
						/>
						<span className="w-[34px] text-right text-[12px] tabular-nums text-ink-3">{faults[key]}%</span>
					</SettingRow>
				))}
				<SettingRow label="" className="justify-end">
					<Button variant="outline" onClick={resetFaults} disabled={!anyFault}>
						Reset all faults
					</Button>
				</SettingRow>
			</Section>
		</div>
	);
}
