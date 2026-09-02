import { useId, useState, type JSX } from 'react';

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
import { Input } from '@/components/ui/input';

import { useCheckRemoteHealth } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { PairingCodeField } from './pairing-code-field';
import { emptyPairingCode, isComplete, pairingCodeValue, type PairingCodeDigits } from '../lib/pairing-code';

export interface AddConnectionSubmission {
	name: string;
	url: string;
	code: string;
}

export interface AddConnectionDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onSubmit: (submission: AddConnectionSubmission) => void;
	busy: boolean;
}

type Stage = 'details' | 'pairing';

/** A daemon worth pairing with is one worth reaching first: stage one collects
 *  Name and URL and health-checks the URL before stage two ever asks for a
 *  pairing code that only the right machine could accept anyway. */
export function AddConnectionDialog({ open, onOpenChange, onSubmit, busy }: AddConnectionDialogProps): JSX.Element {
	const { t } = useTranslation();
	const nameId = useId();
	const urlId = useId();

	const [stage, setStage] = useState<Stage>('details');
	const [name, setName] = useState('');
	const [url, setUrl] = useState('');
	const [code, setCode] = useState<PairingCodeDigits>(() => emptyPairingCode());
	const [healthError, setHealthError] = useState(false);

	const [wasOpen, setWasOpen] = useState(open);
	if (open !== wasOpen) {
		setWasOpen(open);
		if (open) {
			setStage('details');
			setName('');
			setUrl('');
			setCode(emptyPairingCode());
			setHealthError(false);
		}
	}

	const checkHealth = useCheckRemoteHealth();

	const canContinue = name.trim() !== '' && url.trim() !== '' && !checkHealth.isPending;
	const canSubmit = isComplete(code) && !busy;

	async function goToPairing() {
		if (!canContinue) return;
		setHealthError(false);
		try {
			await checkHealth.mutateAsync({ url: url.trim() });
			setStage('pairing');
		} catch {
			setHealthError(true);
		}
	}

	function submit() {
		if (!canSubmit) return;
		onSubmit({ name: name.trim(), url: url.trim(), code: pairingCodeValue(code) });
	}

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>{t('remote.add.title')}</DialogTitle>
					<DialogDescription>
						{stage === 'details' ? t('remote.add.description') : t('remote.add.descriptionPairing')}
					</DialogDescription>
				</DialogHeader>
				<DialogPanel>
					{stage === 'details' ? (
						<div className="flex flex-col gap-3.5">
							<div className="flex flex-col gap-1.5">
								<label className="text-[12.5px] font-medium" htmlFor={nameId}>
									{t('remote.add.name.label')}
								</label>
								<Input
									id={nameId}
									onChange={(event) => setName(event.target.value)}
									placeholder={t('remote.add.name.placeholder')}
									value={name}
								/>
							</div>
							<div className="flex flex-col gap-1.5">
								<label className="text-[12.5px] font-medium" htmlFor={urlId}>
									{t('remote.add.url.label')}
								</label>
								<Input
									className="font-mono"
									id={urlId}
									onChange={(event) => {
										setUrl(event.target.value);
										setHealthError(false);
									}}
									placeholder={t('remote.add.url.placeholder')}
									value={url}
								/>
								{healthError && (
									<p className="text-[11.5px] leading-relaxed text-destructive">
										{t('remote.add.healthError')}
									</p>
								)}
							</div>
						</div>
					) : (
						<div className="flex flex-col gap-1.5">
							<span className="text-[12.5px] font-medium">{t('remote.add.code.label')}</span>
							<PairingCodeField aria-label={t('remote.add.code.label')} onChange={setCode} value={code} />
							<p className="text-[11.5px] leading-relaxed text-muted-foreground">
								{t('remote.add.code.hint')}
							</p>
						</div>
					)}
				</DialogPanel>
				<DialogFooter>
					{stage === 'details' ? (
						<>
							<Button onClick={() => onOpenChange(false)} variant="outline">
								{t('remote.add.cancel')}
							</Button>
							<Button
								disabled={!canContinue}
								loading={checkHealth.isPending}
								onClick={() => void goToPairing()}
							>
								{checkHealth.isPending ? t('remote.add.checking') : t('remote.add.continue')}
							</Button>
						</>
					) : (
						<>
							<Button onClick={() => setStage('details')} variant="outline">
								{t('remote.add.back')}
							</Button>
							<Button disabled={!canSubmit} loading={busy} onClick={submit}>
								{busy ? t('remote.add.submitting') : t('remote.add.submit')}
							</Button>
						</>
					)}
				</DialogFooter>
			</DialogPopup>
		</Dialog>
	);
}
