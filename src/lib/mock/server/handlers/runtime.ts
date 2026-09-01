import type { ArrowState, ArrowStepDefinition, StepProgress } from '@/domain/arrow';

import { INSTALL_STEPS, START_STEPS, STOP_STEPS, UNINSTALL_STEPS, UPDATE_STEPS } from '../../world/scenarios/kit';
import { findArrow, MOCK_HOST_PLATFORM, type MockArrow, type MockWorld } from '../../world/types';
import { versioned } from '../../world/types';
import { accepted, fail } from '../envelope';
import { toRuntimeFrame } from '../projections';
import type { Route } from '../router';

const RUNTIME_ENDPOINT = '/v0/runtime';

const STEP_MS = 700;

const STARTABLE: ArrowState[] = ['absent', 'ready', 'running', 'outdated', 'detached', 'removed'];

function pending(steps: ArrowStepDefinition[]): StepProgress[] {
	return steps.map((step, index) => ({ index, title: step.title, status: 'pending', type: 'exec' }));
}

function push(world: MockWorld, arrow: MockArrow): void {
	world.emitter.emit(RUNTIME_ENDPOINT, toRuntimeFrame(arrow));
}

function runSteps(
	world: MockWorld,
	arrow: MockArrow,
	method: string,
	steps: ArrowStepDefinition[],
	variables: Record<string, string>,
	transitional: ArrowState,
	finalState: ArrowState,
	pid?: number
): void {
	const key = versioned(arrow);
	world.cancels.get(key)?.();

	arrow.state = transitional;
	arrow.active_run = { method, variables, steps: pending(steps), ...(pid ? { pid } : {}) };
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

		if (index >= steps.length) {
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
	const arrow = findArrow(world.arrows, ns);
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
					runSteps(world, arrow, 'uninstall', UNINSTALL_STEPS, {}, 'uninstalling', 'absent');
					return accepted();
				}

				case 'stop': {
					// Mirrors quiver.core's real BeginStop.Validate: a detached arrow
					// (a live process the daemon lost track of) is also a valid stop
					// target -- its only real recovery path is a plain, ordinary stop,
					// same call as stopping a running one (see docs/arrow-details-spec.md §9).
					if (arrow.state !== 'running' && arrow.state !== 'detached') {
						return fail(`arrow ${req.params.ns} is not running`, 409);
					}
					runSteps(world, arrow, 'stop', STOP_STEPS, {}, 'stopping', 'ready');
					return accepted();
				}

				case 'update': {
					if (arrow.state !== 'ready' && arrow.state !== 'outdated') {
						return fail(
							`arrow ${req.params.ns} is ${arrow.state}; update only runs from ready/outdated`,
							409
						);
					}
					runSteps(world, arrow, 'update', UPDATE_STEPS, {}, 'updating', 'ready');
					return accepted();
				}

				// The one universal "go" action -- `Target.Lifecycle.Execute`, not a
				// custom method lookup. Real quiver.core hard-gates this to `ready`
				// only, with no manifest override possible (`BeginExecution.Validate`);
				// mirror that exactly rather than checking any method's `available_in`.
				case 'execute': {
					if (arrow.state !== 'ready') {
						return fail(`arrow ${req.params.ns} is ${arrow.state}; execute only runs from ready`, 409);
					}
					const target = arrow.targets.find((t) => t.platform === MOCK_HOST_PLATFORM);
					if (!target) {
						return fail(`arrow ${req.params.ns} declares no target for ${MOCK_HOST_PLATFORM}`, 422);
					}
					runSteps(world, arrow, 'execute', START_STEPS, variables, 'ready', 'running');
					return accepted();
				}

				default: {
					// Any other word is a custom method, invoked by its own name --
					// `POST /v0/runtime/:ns/backup`, not a verb this handler special-cases.
					const target = arrow.targets.find((t) => t.platform === MOCK_HOST_PLATFORM);
					const method = target?.methods[req.params.verb];
					if (!method) {
						return fail(
							`arrow ${req.params.ns} has no method ${req.params.verb} for ${MOCK_HOST_PLATFORM}`,
							404
						);
					}
					if (!method.available_in.includes(arrow.state)) {
						return fail(
							`method ${req.params.verb} is available in ${method.available_in.join('/')}, not ${arrow.state}`,
							409
						);
					}
					runSteps(world, arrow, req.params.verb, method.steps, variables, arrow.state, arrow.state);
					return accepted();
				}
			}
		},
	},
];
