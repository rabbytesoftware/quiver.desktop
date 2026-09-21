#!/usr/bin/env bash
# Stands up the local GitHub stand-in and points the container's resolver at
# it. Must run AFTER every build, because from this moment on nothing in the
# container can reach the real github.com.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATE="${UPSTREAM_STATE:-/workspace/run/upstream}"
RESULTS="${RESULTS:-/workspace/results}"
HOSTS_MARK="# quiver-e2e-upstream"

CORE_SRC="${CORE_SRC:-/workspace/build/src/quiver.core}"
DESK_SRC="${DESK_SRC:-/workspace/build/src/quiver.desktop}"

"$HERE/trust.sh"

mkdir -p "$STATE"/{raw,releases,git} "$RESULTS"

# --- bare repositories the constraint resolver reads -----------------------
#
# A globbed dependency constraint (quiver.desktop's `tools:` edge on
# quiver.core@stable-26.5*) is resolved by graph.resolveEdgeNs ->
# manifold.ResolveConstraint -> go-git's ls-remote against the namespace's own
# clone URL. There is no host API shortcut for that, so a real git remote has
# to exist. Each repo carries its real committed ARROW.md so that a fallback
# clone (the path a host quiver.core does not know would take) finds the same
# manifest the raw endpoint serves.
seed_repo() {
	local repo="$1" manifest="$2" branch="$3"
	local bare="$STATE/git/${repo}.git"
	[ -d "$bare" ] && return 0

	local work
	work="$(mktemp -d)"
	git -C "$work" init -q -b "$branch"
	cp "$manifest" "$work/ARROW.md"
	git -C "$work" add ARROW.md
	git -C "$work" -c user.email=e2e@quiver.local -c user.name='Quiver E2E' \
		commit -qm "seed $repo"

	mkdir -p "$(dirname "$bare")"
	git init -q --bare "$bare"
	git -C "$work" push -q "$bare" "$branch"
	git --git-dir="$bare" symbolic-ref HEAD "refs/heads/$branch"
	rm -rf "$work"
	echo "[upstream] seeded git repo $repo (branch $branch)"
}

seed_repo rabbytesoftware/quiver.core "$CORE_SRC/ARROW.md" develop
seed_repo rabbytesoftware/quiver.desktop "$DESK_SRC/ARROW.md" develop

# --- name resolution -------------------------------------------------------

if ! grep -q "$HOSTS_MARK" /etc/hosts; then
	cat >>/etc/hosts <<EOF
$HOSTS_MARK
127.0.0.1 github.com
127.0.0.1 raw.githubusercontent.com
127.0.0.1 api.github.com
127.0.0.1 objects.githubusercontent.com
127.0.0.1 codeload.github.com
EOF
	echo "[upstream] /etc/hosts now resolves the GitHub hostnames locally"
fi

# --- the server ------------------------------------------------------------

if pgrep -f 'upstream\.py' >/dev/null 2>&1; then
	echo "[upstream] already running"
else
	UPSTREAM_STATE="$STATE" UPSTREAM_LOG="$RESULTS/upstream.log" \
		nohup python3 "$HERE/upstream.py" >"$RESULTS/upstream.stdout.log" 2>&1 &
	echo "[upstream] started (pid $!)"
fi

for _ in $(seq 1 60); do
	if curl -fsS -o /dev/null "https://github.com/rabbytesoftware/quiver.core/info/refs?service=git-upload-pack" 2>/dev/null; then
		echo "[upstream] answering on https://github.com/ with a trusted certificate"
		exit 0
	fi
	sleep 0.5
done

echo "[upstream] never became reachable" >&2
tail -40 "$RESULTS/upstream.stdout.log" >&2 || true
exit 1
