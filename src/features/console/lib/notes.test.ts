import { describe, expect, it } from 'vitest';

import { createTranslator } from '@/lib/i18n';

import type { Note } from './entries';
import { noteText } from './notes';

const { t } = createTranslator('en');

describe('noteText', () => {
	it.each<[Note, string]>([
		[{ type: 'restarted' }, 'Daemon restarted'],
		[{ type: 'gap', dropped: 1 }, '1 line skipped'],
		[{ type: 'gap', dropped: 37 }, '37 lines skipped'],
		[{ type: 'exit', code: 2, error: '' }, 'Exited with code 2'],
		[{ type: 'exit', code: 1, error: 'missing namespace' }, 'Exited with code 1: missing namespace'],
		[{ type: 'refused', status: 403, message: 'not available' }, 'Refused (403): not available'],
		[{ type: 'unsupported' }, 'This daemon has no console.'],
		[{ type: 'helpUnavailable' }, 'The command list is not available.'],
		[{ type: 'text', text: 'connect failed: no socket' }, 'connect failed: no socket'],
	])('%j', (note, want) => expect(noteText(note, t)).toBe(want));
});
