import { createFileRoute } from '@tanstack/react-router';

import { ArrowAppView } from '@/features/arrow-app/components/arrow-app-view';
import { ArrowDetailsScreen } from '@/features/arrow-details/arrow-details-screen';
import { useArrowStore } from '@/lib/core-store';

export const Route = createFileRoute('/arrow/$')({
	component: ArrowPage,
});

function ArrowPage() {
	const { _splat } = Route.useParams();
	const namespace = _splat ?? '';
	const navigate = Route.useNavigate();
	const hasSurface = useArrowStore((s) => Boolean(s.arrows.get(namespace)?.active_run?.surface));
	const onIdentityChange = (next: string) => void navigate({ params: { _splat: next }, replace: true });

	if (hasSurface) return <ArrowAppView namespace={namespace} onIdentityChange={onIdentityChange} />;
	return <ArrowDetailsScreen namespace={namespace} onIdentityChange={onIdentityChange} />;
}
