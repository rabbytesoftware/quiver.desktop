import type { JSX } from 'react';

import { Dialog, DialogDescription, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from '@/components/ui/dialog';

import '../styles/collection.css';

interface UnresolvedArrowsDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	routes: string[];
}

/** Just the bare routes -- there is no `reason` field anywhere in quiver.core's API for why one failed, so there is nothing else honest to show. */
export function UnresolvedArrowsDialog({ open, onOpenChange, routes }: UnresolvedArrowsDialogProps): JSX.Element {
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogPopup>
				<DialogHeader>
					<DialogTitle>Unresolved arrows</DialogTitle>
					<DialogDescription>
						Quiver couldn't resolve these routes -- there's no name, version, or reason to show, only what was asked
						for.
					</DialogDescription>
				</DialogHeader>
				<DialogPanel>
					<div className="collection-unresolved-list">
						{routes.map((route) => (
							<div className="collection-unresolved-row" key={route}>
								<span className="route">{route}</span>
							</div>
						))}
					</div>
				</DialogPanel>
			</DialogPopup>
		</Dialog>
	);
}
