import type { JSX, ReactNode } from 'react';

interface SectionHeaderProps {
	title: string;
	action?: ReactNode;
}

export function SectionHeader({ title, action }: SectionHeaderProps): JSX.Element {
	return (
		<div className="mb-4 flex items-end justify-between border-b border-border pb-2.5">
			<h2 className="text-[13px] font-semibold tracking-[-0.1px]">{title}</h2>
			{action}
		</div>
	);
}
