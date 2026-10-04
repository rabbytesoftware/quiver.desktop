import type { RuntimeUpdate, ActiveRun, ArrowSurface, LastReturn, ArrowState } from '@/domain/arrow';

export interface ActiveRunDTO extends Omit<ActiveRun, 'surface'> {
	/** Absent on a daemon that predates arrow apps, or when the run serves no page. */
	surface?: ArrowSurface | null;
}

export function toActiveRun(dto: ActiveRunDTO | null | undefined): ActiveRun | null {
	if (!dto) return null;
	return { ...dto, surface: dto.surface ?? undefined };
}

export interface RuntimeUpdateDTO {
	namespace: string;
	state: ArrowState;
	active_run?: ActiveRunDTO | null;
	last_return?: LastReturn | null;
}

export function toRuntimeUpdate(dto: RuntimeUpdateDTO): RuntimeUpdate {
	return {
		namespace: dto.namespace,
		state: dto.state,
		active_run: toActiveRun(dto.active_run),
		last_return: dto.last_return ?? null,
	};
}
