import { useState, type JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { NumberField } from '@/components/ui/number-field';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';

import type { ArrowVariable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { EyeIcon, EyeOffIcon } from 'lucide-react';

interface VariableFieldProps {
	variable: ArrowVariable;
	value: string | undefined;
	onChange: (name: string, value: string) => void;
}

/**
 * `sensitive` wins over `type`: a sensitive value is still masked whatever
 * type it is declared as (in practice always `string`, but nothing in the
 * domain type rules the others out).
 */
function VariableField({ variable, value, onChange }: VariableFieldProps): JSX.Element {
	const { t } = useTranslation();
	const [revealed, setRevealed] = useState(false);

	if (variable.sensitive) {
		return (
			<div className="flex items-center gap-1.5">
				<Input
					aria-label={variable.name}
					className="w-40"
					onChange={(event) => onChange(variable.name, event.target.value)}
					type={revealed ? 'text' : 'password'}
					value={value ?? variable.default ?? ''}
				/>
				<Button
					aria-label={revealed ? t('arrow.settings.hide') : t('arrow.settings.reveal')}
					onClick={() => setRevealed((current) => !current)}
					size="icon-sm"
					type="button"
					variant="ghost"
				>
					{revealed ? <EyeOffIcon /> : <EyeIcon />}
				</Button>
			</div>
		);
	}

	if (variable.type === 'boolean') {
		return (
			<Switch
				aria-label={variable.name}
				checked={value === 'true'}
				onCheckedChange={(checked) => onChange(variable.name, String(checked))}
			/>
		);
	}

	if (variable.type === 'select') {
		return (
			<Select
				onValueChange={(next) => onChange(variable.name, next ?? '')}
				value={value ?? variable.default ?? ''}
			>
				<SelectTrigger aria-label={variable.name} className="w-40">
					<SelectValue />
				</SelectTrigger>
				<SelectContent>
					{(variable.values ?? []).map((option) => (
						<SelectItem key={option} value={option}>
							{option}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		);
	}

	if (variable.type === 'number') {
		const hasRange = variable.min !== undefined || variable.max !== undefined;
		return (
			<div className="flex flex-col items-end gap-1">
				<NumberField
					aria-label={variable.name}
					max={variable.max}
					min={variable.min}
					onValueChange={(next) => onChange(variable.name, String(next ?? 0))}
					value={Number(value ?? variable.default ?? 0)}
				/>
				{hasRange && (
					<span className="text-[11px] text-muted-foreground">
						{variable.min ?? '−∞'}–{variable.max ?? '∞'}
					</span>
				)}
			</div>
		);
	}

	return (
		<Input
			aria-label={variable.name}
			className="w-40"
			onChange={(event) => onChange(variable.name, event.target.value)}
			type="text"
			value={value ?? variable.default ?? ''}
		/>
	);
}

interface VariablesFieldListProps {
	variables: ArrowVariable[];
	values: Record<string, string>;
	onChange: (name: string, value: string) => void;
}

/**
 * The row list shared by every surface that lets someone edit an arrow's
 * variables -- the Settings tab, the Settings dialog, and the "Configure"
 * link inside a step preview. One field per declared variable, keyed by
 * name, each rendered by the type it declares.
 */
export function VariablesFieldList({ variables, values, onChange }: VariablesFieldListProps): JSX.Element {
	return (
		<div className="flex flex-col divide-y divide-border">
			{variables.map((variable) => (
				<div
					className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 py-3 first:pt-0 last:pb-0"
					key={variable.name}
				>
					<div className="min-w-0 flex-1">
						<p className="text-sm font-medium">{variable.name}</p>
						{variable.description && (
							<p className="mt-0.5 text-xs text-muted-foreground">{variable.description}</p>
						)}
					</div>
					<div className="shrink-0">
						<VariableField onChange={onChange} value={values[variable.name]} variable={variable} />
					</div>
				</div>
			))}
		</div>
	);
}
