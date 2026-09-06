import { createFileRoute } from '@tanstack/react-router';

import { RemoteScreen } from '@/features/remote';

export const Route = createFileRoute('/remote')({
	component: RemoteScreen,
});
