#!/bin/sh
# Read generated fixtures as data to avoid ETXTBSY during parallel spawning.
exec /bin/sh "$0.body" "$@"
