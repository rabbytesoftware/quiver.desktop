import type { JSX } from 'react';

import { Dialog, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';

import { TriangleAlertIcon } from 'lucide-react';

interface MessageModalProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	title: string;
	message: string;
}

/**
 * A small generic title+message dialog -- the click target for a "problem"
 * chip explaining why an install failed or why an arrow is detached. Unlike
 * `StepYamlModal`, this one genuinely represents a problem, so the warning
 * icon in the header is appropriate here.
 */
export function MessageModal({ open, onOpenChange, title, message }: MessageModalProps): JSX.Element {
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
			</DialogPopup>
		</Dialog>
	);
}
