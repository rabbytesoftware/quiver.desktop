import type { RuntimeUpdate, ActiveRun, LastOutcome, ArrowState } from '@/domain/arrow';

export interface RuntimeUpdateDTO {
	namespace: string;
	state: ArrowState;
	active_run?: ActiveRun | null;
	last_return?: LastOutcome | null;
}

export function toRuntimeUpdate(dto: RuntimeUpdateDTO): RuntimeUpdate {
	return {
		namespace: dto.namespace,
		state: dto.state,
		active_run: dto.active_run ?? null,
		last_outcome: dto.last_return ?? null,
	};
}
