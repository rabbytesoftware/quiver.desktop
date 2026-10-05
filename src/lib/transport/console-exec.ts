import { Channel, invoke } from '@tauri-apps/api/core';

export interface ConsoleRun {
	/** Abandons the run. The daemon cancels the command when the connection closes. */
	cancel(): void;
}

/**
 * The IPC half of `Backend.execConsole`: starts `console_exec` and routes its
 * channel to `onFrame`. A failure to start (no such command, no connection) is
 * delivered as an `error` frame, so a caller only ever has one place to look.
 */
export function startConsoleExec(line: string, onFrame: (frame: string) => void): ConsoleRun {
	const execId = crypto.randomUUID();
	let cancelled = false;

	const channel = new Channel<string>();
	channel.onmessage = (frame) => {
		if (!cancelled) onFrame(frame);
	};

	invoke('console_exec', { execId, line, onFrame: channel }).catch((err: unknown) => {
		if (cancelled) return;
		onFrame(JSON.stringify({ type: 'error', status: 0, message: String(err) }));
	});

	return {
		cancel() {
			if (cancelled) return;
			cancelled = true;
			void invoke('console_exec_cancel', { execId }).catch(() => {});
		},
	};
}
