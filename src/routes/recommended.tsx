import { createFileRoute } from '@tanstack/react-router';

import { RecommendedScreen } from '@/features/recommended/recommended-screen';

export const Route = createFileRoute('/recommended')({
	component: RecommendedScreen,
});
