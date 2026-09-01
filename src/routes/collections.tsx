import { createFileRoute } from '@tanstack/react-router';

import { CollectionsScreen } from '@/features/collections/collections-screen';

export const Route = createFileRoute('/collections')({
	component: CollectionsScreen,
});
