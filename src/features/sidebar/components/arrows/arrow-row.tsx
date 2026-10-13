import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import { InferredBadge } from '@/components/inferred-badge';

import type { ArrowEntry } from '@/domain/arrow';
import { isInferred } from '@/domain/arrow';
import { blockReselect } from '@/features/sidebar/lib/reselect';
import { ROW_ACTIVE, ROW_BASE, ROW_INACTIVE, ROW_SUBLABEL } from '@/features/sidebar/lib/row-base';
import { useArrowContextMenu } from '@/features/sidebar/lib/use-arrow-context-menu';
import { cn } from '@/lib/cn';
import { splitNamespace } from '@/lib/namespace';

import { ArrowIcon } from './arrow-icon';

const ROW = cn(ROW_BASE, ROW_INACTIVE, ROW_ACTIVE, 'group');

const SUBTITLE = cn(ROW_SUBLABEL, 'hidden group-data-[status=active]:flex');

interface ArrowRowProps {
	arrow: ArrowEntry;
}

export function ArrowRow({ arrow }: ArrowRowProps): JSX.Element {
	const onContextMenu = useArrowContextMenu(arrow);
	const { head, tail } = splitNamespace(arrow.namespace);
	// The identity names what the row follows (`@stable`, `@v1.*`); the
	// version is the ref that resolved to, worth showing only when different.
	const resolved = arrow.version && arrow.version !== tail.slice(1) ? arrow.version : null;

	return (
		<Link
			to="/arrow/$"
			params={{ _splat: arrow.namespace }}
			onClick={blockReselect}
			onContextMenu={onContextMenu}
			className={ROW}
		>
			<ArrowIcon namespace={arrow.namespace} name={arrow.name} icon={arrow.icon} />
			<span className="flex min-w-0 flex-1 flex-col justify-center">
				<span className="flex min-w-0 items-center gap-1.5">
					<span data-slot="arrow-name" className="truncate text-[13px]/[16px]">
						{arrow.name}
					</span>
					{isInferred(arrow) && <InferredBadge compact confidence={arrow.confidence} />}
				</span>
				<span data-slot="arrow-namespace" className={SUBTITLE}>
					<span className="truncate">{head}</span>
					<span className="shrink-0">{tail}</span>
					{resolved && <span className="shrink-0">&nbsp;· {resolved}</span>}
				</span>
			</span>
		</Link>
	);
}
