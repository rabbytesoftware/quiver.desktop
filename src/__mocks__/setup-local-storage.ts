import { JSDOM } from 'jsdom';

const { window } = new JSDOM('', { url: 'http://localhost:3000' });

Object.defineProperty(globalThis, 'localStorage', {
	value: window.localStorage,
	configurable: true,
});
