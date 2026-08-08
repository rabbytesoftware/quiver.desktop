import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/remote')({
	component: RemotePage,
});

/** A destination with nothing in it yet. The rail needs somewhere to point —
 *  what Remote shows is the content column's work, not the shell's. */
function RemotePage() {
	return <div data-testid="remote-page" />;
}
