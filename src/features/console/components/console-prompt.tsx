import { useEffect, useRef, type FormEvent, type JSX, type KeyboardEvent } from 'react';


import { completeLine } from '@/features/console/lib/commands';
import { useConsoleStore } from '@/features/console/stores/console-store';
import { useTranslation } from '@/lib/i18n';

/** The keys the prompt claims. Everything else is the input's. */
function onKeyDown(event: KeyboardEvent<HTMLInputElement>): void {
	if (event.nativeEvent.isComposing) return;
	const store = useConsoleStore.getState();
	switch (event.key) {
		case 'ArrowUp':
			event.preventDefault();
			store.stepHistory('up');
			break;
		case 'ArrowDown':
			event.preventDefault();
			store.stepHistory('down');
			break;
		case 'Tab': {
			const result = completeLine(store.draft, store.commands);
			if (result.candidates.length === 0) return;
			event.preventDefault();
			if (result.line !== store.draft) store.setDraft(result.line);
			else if (result.candidates.length > 1) {
				store.push([{ kind: 'out', stream: 'stdout', text: result.candidates.join('  ') }]);
			}
			break;
		}
		case 'Escape':
			event.preventDefault();
			store.setOpen(false);
			break;
	}
}

export interface ConsolePromptProps {
	onSubmit: (line: string) => void;
	/** Becomes true when the console is shown, which is when the prompt takes focus. */
	focused: boolean;
}

/**
 * The command line. Enter runs, Up and Down walk the history, Tab completes a
 * command word from the daemon's own table, and Escape closes the console.
 */
export function ConsolePrompt({ onSubmit, focused }: ConsolePromptProps): JSX.Element {
	const { t } = useTranslation();
	const input = useRef<HTMLInputElement>(null);

	const draft = useConsoleStore((s) => s.draft);
	const setDraft = useConsoleStore((s) => s.setDraft);
	const stream = useConsoleStore((s) => s.stream);

	useEffect(() => {
		if (focused) input.current?.focus({ preventScroll: true });
	}, [focused]);

	function submit(event: FormEvent): void {
		event.preventDefault();
		onSubmit(draft);
	}

	return (
		<form
			onSubmit={submit}
			data-slot="console-prompt"
			className="flex shrink-0 items-center gap-2.5 border-t px-4 focus-within:bg-console-row-hover"
		>
			<label htmlFor="console-input" className="font-mono text-sm">
				<span aria-hidden="true">›</span>
				<span className="sr-only">{t('console.prompt.label')}</span>
			</label>
			<input
				ref={input}
				id="console-input"
				type="text"
				value={draft}
				onChange={(event) => setDraft(event.target.value)}
				onKeyDown={onKeyDown}
				placeholder={t('console.prompt.placeholder')}
				autoComplete="off"
				autoCapitalize="off"
				autoCorrect="off"
				spellCheck={false}
				className="h-[34px] min-w-0 flex-1 bg-transparent font-mono text-[13px] text-console-foreground outline-none placeholder:text-console-dim placeholder:opacity-70"
			/>
			{stream === 'reconnecting' && (
				<span role="status" className="font-mono text-[11px] text-console-dim">
					{t('console.stream.reconnecting')}
				</span>
			)}
		</form>
	);
}
