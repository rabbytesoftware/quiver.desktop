import { useEffect, useId, useState, type JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Dialog, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';

import { useTranslation } from '@/lib/i18n';

export interface RenameConnectionDialogProps {
	open: boolean;
	initialName: string;
	onOpenChange: (open: boolean) => void;
	onSubmit: (name: string) => void;
}

export function RenameConnectionDialog({
	open,
	initialName,
	onOpenChange,
	onSubmit,
}: RenameConnectionDialogProps): JSX.Element {
	const { t } = useTranslation();
	const nameId = useId();
	const [name, setName] = useState(initialName);

	useEffect(() => {
		if (open) setName(initialName);
	}, [open, initialName]);

	const canSubmit = name.trim() !== '';

	function submit() {
		if (!canSubmit) return;
		onSubmit(name.trim());
	}

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>{t('remote.rename.title')}</DialogTitle>
				</DialogHeader>
				<DialogPanel>
					<div className="flex flex-col gap-1.5">
						<label className="text-[12.5px] font-medium" htmlFor={nameId}>
							{t('remote.rename.label')}
						</label>
						<Input id={nameId} onChange={(event) => setName(event.target.value)} value={name} />
					</div>
				</DialogPanel>
				<DialogFooter>
					<Button onClick={() => onOpenChange(false)} variant="outline">
						{t('remote.rename.cancel')}
					</Button>
					<Button disabled={!canSubmit} onClick={submit}>
						{t('remote.rename.submit')}
					</Button>
				</DialogFooter>
			</DialogPopup>
		</Dialog>
	);
}
