import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/arrow/$')({
	component: ArrowPage,
});

function ArrowPage() {
	const { _splat } = Route.useParams();

	return <div data-testid="arrow-page" data-namespace={_splat} />;
}
