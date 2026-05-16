import { vi } from 'vitest';

export const invoke = vi.fn();
export const listen = vi.fn(() => Promise.resolve(() => {}));
