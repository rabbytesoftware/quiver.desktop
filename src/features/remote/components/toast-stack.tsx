import type { JSX } from 'react';

import { CheckIcon } from 'lucide-react';

import { useRemoteStore } from '../stores/remote-store';

export function ToastStack(): JSX.Element {
	const toasts = useRemoteStore((s) => s.toasts);

	return (
		<div className="fixed right-5 bottom-5 z-60 flex flex-col items-end gap-2">
			{toasts.map((toast) => (
				<div
					className="flex items-center gap-2 rounded-lg border border-border bg-popover px-3.5 py-2.5 text-[12.5px] text-popover-foreground shadow-lg/5"
					key={toast.id}
					role="status"
				>
					<CheckIcon aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
					{toast.message}
				</div>
			))}
		</div>
	);
}
