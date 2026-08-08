import { createFileRoute } from '@tanstack/react-router';

/**
 * A SPLAT, not `/arrow/$namespace`. An arrow's key is one identifier that
 * happens to contain slashes and an `@` — `github.com/rabbyte/minecraft@v1.21.4`
 * — so a named param would have to be encoded into every link and decoded out of
 * every read, and would show up percent-mangled in the devtools. `$` carries it
 * through verbatim.
 */
export const Route = createFileRoute('/arrow/$')({
	component: ArrowPage,
});

function ArrowPage() {
	const { _splat } = Route.useParams();

	// On the element rather than in the body: the page is a placeholder, and
	// this is what proves the namespace survived the URL intact.
	return <div data-testid="arrow-page" data-namespace={_splat} />;
}
