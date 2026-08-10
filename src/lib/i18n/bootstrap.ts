import { currentLocale, localeOf, useLocaleStore } from './store';

export function installLocaleSync(): () => void {
	applyDocumentLanguage(currentLocale());

	const unsubscribe = useLocaleStore.subscribe((state) => applyDocumentLanguage(localeOf(state)));

	const onLanguageChange = () => useLocaleStore.getState().redetect();
	window.addEventListener('languagechange', onLanguageChange);

	return () => {
		unsubscribe();
		window.removeEventListener('languagechange', onLanguageChange);
	};
}

function applyDocumentLanguage(locale: string): void {
	document.documentElement.lang = locale;
}
