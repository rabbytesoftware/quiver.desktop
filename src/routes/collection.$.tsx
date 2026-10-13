import { createFileRoute } from '@tanstack/react-router';

import { CollectionDetailScreen } from '@/features/collection';

export const Route = createFileRoute('/collection/$')({
	component: CollectionPage,
});

function CollectionPage() {
	const { _splat } = Route.useParams();
	return <CollectionDetailScreen namespace={_splat ?? ''} />;
}
