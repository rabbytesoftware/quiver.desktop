import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/')({
	component: HomePage,
});

/** A destination with nothing in it yet — see `remote.tsx`. */
function HomePage() {
	return <div data-testid="home-page" />;
}
