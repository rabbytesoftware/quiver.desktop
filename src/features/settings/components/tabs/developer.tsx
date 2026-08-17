import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { NumberField } from '@/components/ui/number-field';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';

import { useTranslation } from '@/lib/i18n';
import { mockForcedByEnv } from '@/lib/mock/preference';
import { FAULT_KEYS, useMockStore } from '@/lib/mock/store';
import { SCENARIOS } from '@/lib/mock/world/scenarios';
import type { ScenarioName } from '@/lib/mock/world/types';

import { Section, SettingRow } from '../section';

export function DeveloperSettings() {
	const { t, formatPercent } = useTranslation();
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
	const applyAndReload = useMockStore((s) => s.applyAndReload);

	const forcedByEnv = mockForcedByEnv();

	const [pending, setPending] = useState<ScenarioName>(scenario);
	const scenarioChanged = pending !== scenario;

	return (
		<div>
			<Section title={t('settings.developer.mock.title')}>
				<SettingRow
					label={t('settings.developer.mock.toggle')}
					description={
						forcedByEnv ? t('settings.developer.mock.forced') : t('settings.developer.mock.reloads')
					}
				>
					<Switch
						checked={enabled || forcedByEnv}
						disabled={forcedByEnv}
						onCheckedChange={(next) => applyAndReload({ enabled: next })}
						aria-label={t('settings.developer.mock.toggle')}
					/>
				</SettingRow>

				<SettingRow
					label={t('settings.developer.mock.scenario')}
					description={SCENARIOS.find((s) => s.name === pending)?.summary}
				>
					<Select
						items={SCENARIOS.map((s) => ({ value: s.name, label: s.label }))}
						value={pending}
						onValueChange={(v) => setPending(v as ScenarioName)}
					>
						<SelectTrigger className="w-[132px]" aria-label={t('settings.developer.mock.scenarioLabel')}>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{SCENARIOS.map((s) => (
								<SelectItem key={s.name} value={s.name}>
									{s.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
					<Button
						size="sm"
						disabled={!scenarioChanged}
						onClick={() => applyAndReload({ enabled: true, scenario: pending })}
					>
						{t('settings.developer.mock.apply')}
					</Button>
				</SettingRow>
			</Section>

			<Section title={t('settings.developer.chaos.title')}>
				<SettingRow
					label={t('settings.developer.chaos.latency')}
					description={t('settings.developer.chaos.latencyDescription')}
					onReset={() => setLatency(0)}
					canReset={latency !== 0}
				>
					<NumberField
						value={latency}
						onValueChange={(v) => setLatency(v ?? 0)}
						min={0}
						max={10000}
						step={50}
						suffix="ms"
						aria-label={t('settings.developer.chaos.latencyLabel')}
					/>
				</SettingRow>

				<SettingRow
					label={t('settings.developer.chaos.errorRate')}
					description={t('settings.developer.chaos.errorRateDescription')}
					onReset={() => setErrorRate(0)}
					canReset={errorRate !== 0}
				>
					<NumberField
						value={errorRate}
						onValueChange={(v) => setErrorRate(v ?? 0)}
						min={0}
						max={100}
						step={5}
						suffix="%"
						aria-label={t('settings.developer.chaos.errorRateLabel')}
					/>
				</SettingRow>

				<SettingRow
					label={t('settings.developer.chaos.unreachable')}
					description={t('settings.developer.chaos.unreachableDescription')}
					onReset={() => setUnreachable(false)}
					canReset={unreachable}
				>
					<Switch
						checked={unreachable}
						onCheckedChange={setUnreachable}
						aria-label={t('settings.developer.chaos.unreachable')}
					/>
				</SettingRow>
			</Section>

			<Section title={t('settings.developer.faults.title')}>
				{FAULT_KEYS.map((key) => (
					<SettingRow
						key={key}
						label={t(`settings.developer.faults.${key}`)}
						onReset={() => setFault(key, 0)}
						canReset={faults[key] > 0}
					>
						<Slider
							value={faults[key]}
							onValueChange={(v) => setFault(key, Array.isArray(v) ? (v[0] ?? 0) : v)}
							step={5}
							className="w-[140px]"
							aria-label={t('settings.developer.faults.rateLabel', {
								family: t(`settings.developer.faults.${key}`),
							})}
						/>
						<span className="w-[34px] text-right text-xs tabular-nums text-muted-foreground">
							{formatPercent(faults[key] / 100)}
						</span>
					</SettingRow>
				))}
			</Section>
		</div>
	);
}
