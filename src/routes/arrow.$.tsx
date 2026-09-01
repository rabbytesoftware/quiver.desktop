import { createFileRoute } from '@tanstack/react-router';

import { ArrowDetailsScreen } from '@/features/arrow-details/arrow-details-screen';

export const Route = createFileRoute('/arrow/$')({
	component: ArrowPage,
});

function ArrowPage() {
	const { _splat } = Route.useParams();

	return <ArrowDetailsScreen namespace={_splat ?? ''} />;
}
