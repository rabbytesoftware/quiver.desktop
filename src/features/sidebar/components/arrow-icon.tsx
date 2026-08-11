import type { JSX } from 'react';

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import { ROW_GLYPH_BOX } from '../row-base';

function chipHue(namespace: string): number {
	let hash = 0x811c9dc5;
	for (let index = 0; index < namespace.length; index += 1) {
		hash = Math.imul(hash ^ namespace.charCodeAt(index), 0x01000193);
	}
	return (hash >>> 0) % 360;
}

function chipColour(namespace: string): string {
	return `oklch(0.52 0.15 ${chipHue(namespace)})`;
}

function monogram(name: string): string {
	const words = name.split(/\s+/).filter((word) => word.length > 0);
	const glyphs = words.length > 1 ? words.map((word) => Array.from(word)[0]) : Array.from(words[0] ?? '');
	return glyphs.slice(0, 2).join('');
}

const AVATAR = cn(ROW_GLYPH_BOX, 'overflow-hidden rounded-sm bg-transparent');

const MONOGRAM =
	'size-full rounded-none bg-transparent text-[8.5px] font-[700] leading-(--icon) tracking-[0] text-white uppercase';

interface ArrowIconProps {
	namespace: string;
	name: string;
	icon: string | null;
}

export function ArrowIcon({ namespace, name, icon }: ArrowIconProps): JSX.Element {
	const { t } = useTranslation();

	return (
		<Avatar className={AVATAR} data-slot="arrow-icon">
			{icon !== null && <AvatarImage src={icon} alt="" className="object-cover" />}
			<AvatarFallback
				data-slot="arrow-monogram"
				role="img"
				aria-label={t('arrow.icon.fallback', { name })}
				className={MONOGRAM}
				style={{ backgroundColor: chipColour(namespace) }}
			>
				{monogram(name)}
			</AvatarFallback>
		</Avatar>
	);
}
