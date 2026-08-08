// The shape of a message catalogue, and the type machinery that turns a typo
// into a compile error instead of a string that quietly renders as its own key.
//
// Nothing in here has a runtime cost: it is all `type`, erased by tsc. The
// price is paid once, at build time, by the same `tsc` run that `bun run build`
// already performs.

/**
 * A message with more than one grammatical number.
 *
 * `other` is required and every other form is optional, which is the only
 * arrangement that is correct for all locales at once: English needs `one` and
 * `other`, Russian needs `one`, `few`, `many` and `other`, Welsh and Arabic use
 * all six. A locale author fills in the forms their language has and
 * `selectPluralForm` falls back to `other` for the rest — so a catalogue that
 * is merely incomplete reads awkwardly rather than throwing.
 *
 * The CLDR category names come from `Intl.LDMLPluralRule`, not from a list
 * written out here: `Intl.PluralRules.select` returns exactly those strings, so
 * borrowing the type is what keeps the lookup in `selectPluralForm` total.
 */
export type PluralMessage = { other: string } & Partial<Record<Exclude<Intl.LDMLPluralRule, 'other'>, string>>;

export type Message = string | PluralMessage;

export type Catalogue = Readonly<Record<string, Message>>;

/** Values are `string | number` and not `unknown` on purpose — see `interpolate`. */
export type MessageParams = Readonly<Record<string, string | number>>;

/**
 * Every `{name}` in a string literal type, as a union of the names.
 *
 * `'{count} of {total}'` → `'count' | 'total'`. Recursive on the tail so a
 * message with several placeholders yields all of them; a message with none
 * yields `never`, which is what `ParamsFor` tests for to decide whether the
 * params argument exists at all.
 */
type PlaceholdersIn<S extends string> = S extends `${string}{${infer Name}}${infer Rest}`
	? Name | PlaceholdersIn<Rest>
	: never;

/**
 * The same, for a whole message — including a plural one, where the
 * placeholders are spread across the forms and the union of all of them is what
 * a caller has to supply. (They are the same set in practice, because a form
 * that drops `{count}` is a translation bug; taking the union rather than the
 * intersection means the type does not silently stop asking for a parameter
 * that only one form uses.)
 *
 * Deliberately unconstrained: it is applied to `(typeof en)[K]`, whose literal
 * object type has every present form as a REQUIRED property. Constraining it to
 * `Message` would widen the optional forms of `PluralMessage` to
 * `string | undefined`, which fails `M[K] extends string` and drops every
 * placeholder on the floor.
 */
type PlaceholdersOf<M> = M extends string
	? PlaceholdersIn<M>
	: M extends object
		? { [K in keyof M]: M[K] extends string ? PlaceholdersIn<M[K]> : never }[keyof M]
		: never;

/**
 * What `t` demands alongside a key.
 *
 *  - a plain string with no placeholders → `undefined`, i.e. no second argument;
 *  - a plain string with placeholders → exactly those, nothing missing;
 *  - a plural message → those, plus `count`, which is what picks the form.
 *
 * `[X] extends [never]` rather than `X extends never`: a bare conditional over
 * a naked type parameter distributes, and distributing over `never` produces
 * `never` for BOTH branches — the tuple wrapper is what stops that.
 */
export type ParamsFor<M> = M extends string
	? [PlaceholdersOf<M>] extends [never]
		? undefined
		: Readonly<Record<PlaceholdersOf<M>, string | number>>
	: Readonly<{ count: number } & Record<Exclude<PlaceholdersOf<M>, 'count'>, string | number>>;

/**
 * The rest-parameter tuple for `t`. An empty tuple where a message takes no
 * parameters, so `t('settings.title', {})` is a type error rather than a habit
 * that hides a later `{name}` being added without its argument.
 */
export type TranslateArgs<M> = [ParamsFor<M>] extends [undefined] ? [] : [params: ParamsFor<M>];
