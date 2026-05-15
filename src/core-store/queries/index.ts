export { useArrowDetail } from './arrows';
export { useCollections, useCollectionDetail } from './collections';

export const queryKeys = {
	arrowDetail: (namespace: string) => ['arrow', 'detail', namespace] as const,
	collections: () => ['collections'] as const,
	collectionDetail: (namespace: string) => ['collection', 'detail', namespace] as const,
};
