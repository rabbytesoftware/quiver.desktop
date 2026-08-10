import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/remote')({
	component: RemotePage,
});

function RemotePage() {
	return <div data-testid="remote-page" />;
}
