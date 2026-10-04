import { useEffect } from 'react';

import { createFileRoute } from '@tanstack/react-router';

import { ArrowAppEmpty } from '@/features/arrow-app/components/arrow-app-empty';
import { ArrowAppHeader } from '@/features/arrow-app/components/arrow-app-header';
import { useOpenApps } from '@/features/arrow-app/store/open-apps';
import { useArrowStore } from '@/lib/core-store';

export const Route = createFileRoute('/app/$')({
	component: AppPage,
});

function AppPage() {
	const { _splat } = Route.useParams();
	const namespace = _splat ?? '';
	const live = useArrowStore((s) => Boolean(s.arrows.get(namespace)?.active_run?.surface));

	useEffect(() => {
		if (live) useOpenApps.getState().show(namespace);
		return () => useOpenApps.getState().hide();
	}, [namespace, live]);

	if (!live) return <ArrowAppEmpty namespace={namespace} />;
	return <ArrowAppHeader namespace={namespace} onReload={() => useOpenApps.getState().reload(namespace)} />;
}
