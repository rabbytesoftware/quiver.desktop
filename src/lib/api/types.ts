/** Standard envelope for all quiver.core responses */
export interface ApiResponse<T = unknown> {
	success: boolean;
	error: string | null;
	data?: T;
}

/** Mutation responses return namespace instead of data */
export interface ApiMutationResponse {
	success: boolean;
	error: string | null;
	namespace: string;
}
