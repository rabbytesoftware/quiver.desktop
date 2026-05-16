import type { RuntimeUpdate, ActiveRun, LastReturn, ArrowState } from '@/domain/arrow';

export interface RuntimeUpdateDTO {
	namespace: string;
	state: ArrowState;
	active_run?: ActiveRun | null;
	last_return?: LastReturn | null;
}

export function toRuntimeUpdate(dto: RuntimeUpdateDTO): RuntimeUpdate {
	return {
		namespace: dto.namespace,
		state: dto.state,
		active_run: dto.active_run ?? null,
		last_return: dto.last_return ?? null,
	};
}
