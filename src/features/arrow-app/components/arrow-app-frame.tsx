import type { JSX } from 'react';

import { cn } from '@/lib/cn';

import { arrowAppOrigin } from '../lib/surface';

interface ArrowAppFrameProps {
	host: string;
	path: string;
	windows: boolean;
	visible: boolean;
	reloadKey: number;
	onRef: (el: HTMLIFrameElement | null) => void;
}

/**
 * One arrow's page. The sandbox keeps it from navigating the shell or opening
 * windows; `allow-same-origin` is safe because the page's origin is its own
 * `arrow-app` host, never the shell's. Hidden frames stay mounted so the page
 * keeps its state and connections.
 */
export function ArrowAppFrame({ host, path, windows, visible, reloadKey, onRef }: ArrowAppFrameProps): JSX.Element {
	return (
		<iframe
			key={reloadKey}
			ref={onRef}
			title={host}
			src={`${arrowAppOrigin(host, windows)}${path}`}
			sandbox="allow-scripts allow-same-origin"
			referrerPolicy="no-referrer"
			className={cn('pointer-events-auto absolute inset-0 size-full border-0 bg-background', !visible && 'hidden')}
		/>
	);
}
