import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

interface ArrowListNoticeProps {
	title: string;
	body: string;
	// The `tab` union mirrors `SettingsTab` (src/features/settings/store.ts)
	// without importing it — the sidebar has no other reason to depend on the
	// settings feature, and this is the one deep link it needs.
	action?: { label: string; to: '/settings'; search?: { tab: 'general' | 'engine' | 'developer' } };
}

export function ArrowListNotice({ title, body, action }: ArrowListNoticeProps): JSX.Element {
	return (
		<div className="flex flex-col gap-1 px-4 py-6 text-center" data-slot="arrow-list-notice">
			<p className="text-[13px] font-medium tracking-[-0.1px] text-foreground">{title}</p>
			<p className="text-[11.5px]/[15px] text-muted-foreground text-balance">{body}</p>
			{action && (
				<Link
					className="mt-1 self-center rounded-sm text-[11.5px] font-medium text-foreground underline underline-offset-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
					to={action.to}
					search={action.search}
				>
					{action.label}
				</Link>
			)}
		</div>
	);
}
