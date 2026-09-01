import { useState, type JSX } from 'react';

import { Button } from '@/components/ui/button';
import {
	Dialog,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogPanel,
	DialogPopup,
	DialogTitle,
} from '@/components/ui/dialog';

import type { ArrowVariable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { VariablesFieldList } from './variable-field';

interface VariablesSettingsModalProps {
	variables: ArrowVariable[];
	values: Record<string, string>;
	onChange: (name: string, value: string) => void;
}

/**
 * A "Configure" link that opens the same variable fields as the Settings
 * tab, in a dialog -- for setting values right before an action runs (e.g.
 * from a step preview), without leaving that context to go find the tab.
 */
export function VariablesSettingsModal({ variables, values, onChange }: VariablesSettingsModalProps): JSX.Element {
	const { t } = useTranslation();
	const [open, setOpen] = useState(false);

	return (
		<>
			<Button className="h-auto p-0" onClick={() => setOpen(true)} size="sm" variant="link">
				{t('arrow.preview.configure')}
			</Button>

			<Dialog onOpenChange={setOpen} open={open}>
				<DialogPopup>
					<DialogHeader>
						<DialogTitle>{t('arrow.settings.title')}</DialogTitle>
						<DialogDescription>{t('arrow.settings.subtitle')}</DialogDescription>
					</DialogHeader>
					<DialogPanel>
						<VariablesFieldList onChange={onChange} values={values} variables={variables} />
					</DialogPanel>
					<DialogFooter>
						<Button onClick={() => setOpen(false)}>{t('arrow.settings.done')}</Button>
					</DialogFooter>
				</DialogPopup>
			</Dialog>
		</>
	);
}
