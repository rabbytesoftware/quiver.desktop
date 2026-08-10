export type PluralMessage = { other: string } & Partial<Record<Exclude<Intl.LDMLPluralRule, 'other'>, string>>;

export type Message = string | PluralMessage;

export type Catalogue = Readonly<Record<string, Message>>;

export type MessageParams = Readonly<Record<string, string | number>>;

type PlaceholdersIn<S extends string> = S extends `${string}{${infer Name}}${infer Rest}`
	? Name | PlaceholdersIn<Rest>
	: never;

type PlaceholdersOf<M> = M extends string
	? PlaceholdersIn<M>
	: M extends object
		? { [K in keyof M]: M[K] extends string ? PlaceholdersIn<M[K]> : never }[keyof M]
		: never;

type ParamsFor<M> = M extends string
	? [PlaceholdersOf<M>] extends [never]
		? undefined
		: Readonly<Record<PlaceholdersOf<M>, string | number>>
	: Readonly<{ count: number } & Record<Exclude<PlaceholdersOf<M>, 'count'>, string | number>>;

export type TranslateArgs<M> = [ParamsFor<M>] extends [undefined] ? [] : [params: ParamsFor<M>];
