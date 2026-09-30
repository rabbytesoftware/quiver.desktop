import type { Home } from '@/domain/home';

import type { SearchResultDTO } from './search';
import { toSearchEntry } from './search';

export interface HomeShelfDTO {
	id: string;
	title: string;
	refreshed_at: string | null;
	arrows: SearchResultDTO[] | null;
}

/** `GET /v0/home`. Each arrow is the same DTO `GET /v0/search` returns. */
export interface HomeDTO {
	shelves: HomeShelfDTO[] | null;
	refreshing: boolean;
}

export function toHome(dto: HomeDTO): Home {
	return {
		shelves: (dto.shelves ?? []).map((shelf) => ({
			id: shelf.id,
			title: shelf.title,
			refreshedAt: shelf.refreshed_at ?? null,
			arrows: (shelf.arrows ?? []).map(toSearchEntry),
		})),
		refreshing: dto.refreshing === true,
	};
}
