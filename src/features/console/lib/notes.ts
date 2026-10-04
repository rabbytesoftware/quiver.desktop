import type { Translator } from '@/lib/i18n';

import type { Note } from '../stores/console-store';

/** What the console says about itself, in words. */
export function noteText(note: Note, t: Translator['t']): string {
	switch (note.type) {
		case 'restarted':
			return t('console.note.restarted');
		case 'gap':
			return t('console.note.gap', { count: note.dropped });
		case 'exit':
			return note.error
				? t('console.note.exitWithError', { code: note.code, error: note.error })
				: t('console.note.exit', { code: note.code });
		case 'refused':
			return t('console.note.refused', { status: note.status, message: note.message });
		case 'unsupported':
			return t('console.note.unsupported');
		case 'helpUnavailable':
			return t('console.note.helpUnavailable');
		case 'text':
			return note.text;
	}
}
