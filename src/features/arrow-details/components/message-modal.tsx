import type { JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Dialog, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';

import { TriangleAlertIcon } from 'lucide-react';

interface MessageModalProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	title: string;
	message: string;
	/**
	 * Present only for a warn-then-confirm flow (e.g. adding a
	 * not-supported-on-this-platform arrow to the library anyway) -- renders a
	 * Cancel/Confirm footer instead of the plain dismiss-only shape every other
	 * caller (a failed-run detail, a release error) keeps by leaving these out.
	 * Cancel always just closes the dialog; `onConfirm` is the caller's own
	 * side effect for proceeding.
	 */
	confirmLabel?: string;
	cancelLabel?: string;
	onConfirm?: () => void;
}

/**
 * A small generic title+message dialog -- the click target for a "problem"
 * chip explaining why an install failed or why an arrow is detached. Unlike
 * `StepYamlModal`, this one genuinely represents a problem, so the warning
 * icon in the header is appropriate here.
 */
export function MessageModal({
	open,
	onOpenChange,
	title,
	message,
	confirmLabel,
	cancelLabel,
	onConfirm,
}: MessageModalProps): JSX.Element {
	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogPopup>
				<DialogHeader>
					<div className="flex items-center gap-2">
						<TriangleAlertIcon aria-hidden="true" className="size-4.5 shrink-0 text-destructive" />
						<DialogTitle>{title}</DialogTitle>
					</div>
				</DialogHeader>
				<DialogPanel>
					<pre className="whitespace-pre-wrap break-words font-mono text-xs text-foreground">{message}</pre>
				</DialogPanel>
				{onConfirm && (
					<DialogFooter>
						<Button onClick={() => onOpenChange(false)} variant="outline">
							{cancelLabel}
						</Button>
						<Button onClick={onConfirm} variant="default">
							{confirmLabel}
						</Button>
					</DialogFooter>
				)}
			</DialogPopup>
		</Dialog>
	);
}
