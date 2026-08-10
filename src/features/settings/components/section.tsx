import { createContext, type ReactNode, useCallback, useContext, useEffect, useId, useRef, useState } from 'react';

import { cn } from '@/lib/cn';

import { rowMatchesQuery, useSettingsUI } from '../store';

const RowVisibility = createContext<((rowId: string, visible: boolean | null) => void) | null>(null);

export function Section({
	title,
	description,
	children,
}: {
	title: string;
	description?: string;
	children: ReactNode;
}) {
	const searching = useSettingsUI((s) => s.query.trim().length > 0);
	const visibility = useRef(new Map<string, boolean>());
	const [visibleRows, setVisibleRows] = useState(0);

	const report = useCallback((rowId: string, visible: boolean | null) => {
		if (visible === null) visibility.current.delete(rowId);
		else visibility.current.set(rowId, visible);
		setVisibleRows([...visibility.current.values()].filter(Boolean).length);
	}, []);

	const hidden = searching && visibleRows === 0;

	return (
		<RowVisibility.Provider value={report}>
			<section hidden={hidden} className="mb-6">
				<h3 className="mb-1 text-sm font-medium text-foreground">{title}</h3>
				{description && <p className="mb-2 text-xs leading-relaxed text-muted-foreground">{description}</p>}
				<div>{children}</div>
			</section>
		</RowVisibility.Provider>
	);
}

export function SettingRow({
	label,
	description,
	children,
	className,
}: {
	label: string;
	description?: ReactNode;
	children?: ReactNode;
	className?: string;
}) {
	const query = useSettingsUI((s) => s.query);
	const report = useContext(RowVisibility);
	const rowId = useId();
	const matches = rowMatchesQuery(query, label, typeof description === 'string' ? description : undefined);

	useEffect(() => {
		if (!report) return;
		report(rowId, matches);
		return () => report(rowId, null);
	}, [report, rowId, matches]);

	if (!matches) return null;

	return (
		<div
			className={cn(
				'flex min-h-[34px] items-center justify-between gap-4 px-1 py-1.5 transition-colors hover:bg-accent/50',
				className
			)}
		>
			<div className="min-w-0 flex-1">
				<div className="text-sm leading-tight text-foreground">{label}</div>
				{description && (
					<div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{description}</div>
				)}
			</div>
			{children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
		</div>
	);
}
