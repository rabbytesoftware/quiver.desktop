import { useNavigate } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import type { ArrowSurface } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

interface Props {
	namespace: string;
	surface: ArrowSurface | null | undefined;
}

/** Opens the arrow's interface in the shell, once it answers. */
export function OpenAppButton({ namespace, surface }: Props) {
	const { t } = useTranslation();
	const navigate = useNavigate();
	if (!surface) return null;

	return (
		<Button
			size="sm"
			disabled={!surface.ready}
			onClick={() => void navigate({ to: '/app/$', params: { _splat: namespace } })}
		>
			{t('arrowApp.open')}
		</Button>
	);
}
