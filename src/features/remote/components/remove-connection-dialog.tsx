import type { JSX } from 'react';

import { Button } from '@/components/ui/button';
import {
	Dialog,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogPopup,
	DialogTitle,
} from '@/components/ui/dialog';

import { useTranslation } from '@/lib/i18n';

export interface RemoveConnectionDialogProps {
	open: boolean;
	name: string;
	isActive: boolean;
	onOpenChange: (open: boolean) => void;
	onConfirm: () => void;
}

export function RemoveConnectionDialog({
	open,
	name,
	isActive,
	onOpenChange,
	onConfirm,
}: RemoveConnectionDialogProps): JSX.Element {
	const { t } = useTranslation();

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>{t('remote.remove.title', { name })}</DialogTitle>
					<DialogDescription>{t('remote.remove.description')}</DialogDescription>
					{isActive && <DialogDescription>{t('remote.remove.activeWarning')}</DialogDescription>}
				</DialogHeader>
				<DialogFooter>
					<Button onClick={() => onOpenChange(false)} variant="outline">
						{t('remote.remove.cancel')}
					</Button>
					<Button onClick={onConfirm} variant="destructive">
						{t('remote.remove.submit')}
					</Button>
				</DialogFooter>
			</DialogPopup>
		</Dialog>
	);
}
