export const arrowKeys = {
	all: ['arrows'] as const,
	lists: () => [...arrowKeys.all, 'list'] as const,
	detail: (namespace: string) => [...arrowKeys.all, 'detail', namespace] as const,
};

export const quiverKeys = {
	all: ['quivers'] as const,
	lists: () => [...quiverKeys.all, 'list'] as const,
	detail: (namespace: string) => [...quiverKeys.all, 'detail', namespace] as const,
};
