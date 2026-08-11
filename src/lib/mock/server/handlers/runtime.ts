import type { ArrowState, StepProgress } from '@/domain/arrow';

import { INSTALL_STEPS } from '../../world/scenarios/kit';
import { MOCK_HOST_PLATFORM, type MockArrow, type MockWorld } from '../../world/types';
import { versioned } from '../../world/types';
import { accepted, fail } from '../envelope';
import { toRuntimeFrame } from '../projections';
import type { Route } from '../router';

const RUNTIME_ENDPOINT = '/v0/runtime';

const STEP_MS = 700;

const STARTABLE: ArrowState[] = ['absent', 'ready', 'running', 'outdated', 'detached', 'removed'];

function pending(titles: string[]): StepProgress[] {
	return titles.map((title, index) => ({ index, title, status: 'pending', type: 'exec' }));
}

function push(world: MockWorld, arrow: MockArrow): void {
	world.emitter.emit(RUNTIME_ENDPOINT, toRuntimeFrame(arrow));
}

function runSteps(
	world: MockWorld,
	arrow: MockArrow,
	method: string,
	titles: string[],
	variables: Record<string, string>,
	transitional: ArrowState,
	finalState: ArrowState,
	pid?: number
): void {
	const key = versioned(arrow);
	world.cancels.get(key)?.();

	arrow.state = transitional;
	arrow.active_run = { method, variables, steps: pending(titles), ...(pid ? { pid } : {}) };
	arrow.last_return = null;
	push(world, arrow);

	let index = 0;
	const stop = world.clock.every(STEP_MS, () => {
		const run = arrow.active_run;
		if (!run) {
			stop();
			world.cancels.delete(key);
			return;
		}

		if (index > 0) run.steps[index - 1].status = 'completed';

		if (index >= titles.length) {
			stop();
			world.cancels.delete(key);
			arrow.state = finalState;
			arrow.last_return = { method, outcome: 'success', variables, steps: run.steps };
			arrow.active_run = finalState === 'running' ? run : null;
			if (finalState === 'running') run.pid = pid ?? 40000 + world.nextId();
			push(world, arrow);
			return;
		}

		run.steps[index].status = 'running';
		index++;
		push(world, arrow);
	});

	world.cancels.set(key, () => {
		stop();
		world.cancels.delete(key);
	});
}

function requireArrow(world: MockWorld, ns: string): MockArrow | Response {
	const arrow = world.arrows.get(ns);
	return arrow ?? fail(`arrow ${ns} not found`, 404);
}

export const runtimeRoutes: Route[] = [
	{
		method: 'POST',
		pattern: '/v0/runtime/:ns/:verb',
		fault: 'runtime',
		handler: (req, world) => {
			const found = requireArrow(world, req.params.ns);
			if (found instanceof Response) return found;
			const arrow = found;

			if (!STARTABLE.includes(arrow.state)) {
				return fail(`arrow ${req.params.ns} is ${arrow.state}; no action can start from there`, 409);
			}

			const body = (req.body ?? {}) as { method?: string; variables?: Record<string, string> };
			const variables = body.variables ?? {};

			switch (req.params.verb) {
				case 'install': {
					if (!arrow.targets.some((t) => t.platform === MOCK_HOST_PLATFORM)) {
						return fail(
							`arrow ${req.params.ns} declares no target for ${MOCK_HOST_PLATFORM} ` +
								`(has: ${arrow.targets.map((t) => t.platform).join(', ') || 'none'})`,
							422
						);
					}
					runSteps(world, arrow, 'install', INSTALL_STEPS, variables, 'installing', 'ready');
					return accepted();
				}

				case 'uninstall': {
					runSteps(
						world,
						arrow,
						'uninstall',
						['Stop process', 'Remove workdir', 'Prune runtime config'],
						{},
						'uninstalling',
						'absent'
					);
					return accepted();
				}

				case 'stop': {
					if (arrow.state !== 'running') {
						return fail(`arrow ${req.params.ns} is not running`, 409);
					}
					runSteps(world, arrow, 'stop', ['Signal process', 'Await exit'], {}, 'stopping', 'ready');
					return accepted();
				}

				case '_execute': {
					const name = body.method;
					if (!name) return fail('_execute requires a method name', 400);

					const target = arrow.targets.find((t) => t.platform === MOCK_HOST_PLATFORM);
					const method = target?.methods[name];
					if (!method) {
						return fail(`arrow ${req.params.ns} has no method ${name} for ${MOCK_HOST_PLATFORM}`, 404);
					}
					if (!method.available_in.includes(arrow.state as 'ready' | 'running')) {
						return fail(
							`method ${name} is available in ${method.available_in.join('/')}, not ${arrow.state}`,
							409
						);
					}

					const willRun = name === 'start';
					runSteps(
						world,
						arrow,
						name,
						method.steps,
						variables,
						arrow.state,
						willRun ? 'running' : arrow.state
					);
					return accepted();
				}

				default:
					return fail(`unknown runtime verb ${req.params.verb}`, 404);
			}
		},
	},
];
