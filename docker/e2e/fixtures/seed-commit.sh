#!/usr/bin/env bash
# seed-commit.sh WORKDIR MANIFEST BRANCH
#
# Commits MANIFEST as ARROW.md in a fresh repository at WORKDIR and prints the
# commit. Author, committer, dates and message are fixed, so the same manifest
# always gives the same commit: build.sh stamps it into quiver.core as
# main.commit before upstream-up.sh seeds the stand-in repository with it, and
# a core build whose commit is the commit its tag names is one quiver.core
# reports as current rather than "unknown, so outdated once".
set -euo pipefail

work="$1" manifest="$2" branch="$3"

export GIT_AUTHOR_NAME='Quiver E2E' GIT_AUTHOR_EMAIL=e2e@quiver.local
export GIT_COMMITTER_NAME='Quiver E2E' GIT_COMMITTER_EMAIL=e2e@quiver.local
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'

git -C "$work" init -q -b "$branch"
install -m 0644 "$manifest" "$work/ARROW.md"
git -C "$work" add ARROW.md
git -C "$work" -c commit.gpgsign=false commit -qm seed
git -C "$work" rev-parse HEAD
