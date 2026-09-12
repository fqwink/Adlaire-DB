#!/usr/bin/env sh
set -eu

if command -v docker >/dev/null 2>&1; then
  DOCKER="${DOCKER:-docker}"
elif [ -x /Applications/Docker.app/Contents/Resources/bin/docker ]; then
  export PATH="/Applications/Docker.app/Contents/Resources/bin:$PATH"
  DOCKER="${DOCKER:-/Applications/Docker.app/Contents/Resources/bin/docker}"
else
  echo "docker command not found" >&2
  exit 127
fi

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.test.yml}"

exec "$DOCKER" compose -f "$COMPOSE_FILE" run --rm --build test "$@"
