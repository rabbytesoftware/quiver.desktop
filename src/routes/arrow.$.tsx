import { createFileRoute } from '@tanstack/react-router';

import { ArrowPage } from '@/features/arrow-app/components/arrow-page';

export const Route = createFileRoute('/arrow/$')({
	component: ArrowRoute,
});

function ArrowRoute() {
	const { _splat } = Route.useParams();
	const navigate = Route.useNavigate();
	const onIdentityChange = (next: string) => void navigate({ params: { _splat: next }, replace: true });

	return <ArrowPage namespace={_splat ?? ''} onIdentityChange={onIdentityChange} />;
}
