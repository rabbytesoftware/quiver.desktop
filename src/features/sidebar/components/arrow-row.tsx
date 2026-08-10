import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import type { ArrowEntry } from '@/domain/arrow';
import { cn } from '@/lib/cn';

import { splitNamespace } from '../namespace';
import { blockReselect } from '../reselect';
import { ROW_ACTIVE, ROW_BASE, ROW_INACTIVE, ROW_SUBLABEL } from '../row-base';
import { ArrowIcon } from './arrow-icon';

const ROW = cn(ROW_BASE, ROW_INACTIVE, ROW_ACTIVE, 'group');

const SUBTITLE = cn(ROW_SUBLABEL, 'hidden group-data-[status=active]:flex');

interface ArrowRowProps {
	arrow: ArrowEntry;
}

export function ArrowRow({ arrow }: ArrowRowProps): JSX.Element {
	const { head, tail } = splitNamespace(arrow.namespace);

	return (
		<Link to="/arrow/$" params={{ _splat: arrow.namespace }} onClick={blockReselect} className={ROW}>
			<ArrowIcon namespace={arrow.namespace} name={arrow.name} icon={arrow.icon} />
			<span className="flex min-w-0 flex-1 flex-col justify-center">
				<span data-slot="arrow-name" className="truncate text-[13px]/[16px]">
					{arrow.name}
				</span>
				<span data-slot="arrow-namespace" className={SUBTITLE}>
					<span className="truncate">{head}</span>
					<span className="shrink-0">{tail}</span>
				</span>
			</span>
		</Link>
	);
}
