#!/usr/bin/env bash
# quiver.core, stamped the way the release workflows stamp it, into /perf/bin/quiver (the daemon both apps talk to).
set -euo pipefail
mkdir -p /perf/bin
cd /core
CGO_ENABLED=0 go build \
  -ldflags "-X main.version=stable-26.5.90 -X main.commit=0123456789abcdef0123456789abcdef01234567 -X main.channel=stable -X main.buildID=9000" \
  -o /perf/bin/quiver ./cmd/quiver
/perf/bin/quiver version
