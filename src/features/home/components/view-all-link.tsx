import type { JSX, ReactNode } from 'react';

import { Link } from '@tanstack/react-router';

interface ViewAllLinkProps {
	to: '/library' | '/collections' | '/recommended';
	children: ReactNode;
}

export function ViewAllLink({ to, children }: ViewAllLinkProps): JSX.Element {
	return (
		<Link
			className="inline-flex items-center gap-1 pb-2.5 text-[12px] text-muted-foreground hover:text-foreground"
			to={to}
		>
			{children}
		</Link>
	);
}
