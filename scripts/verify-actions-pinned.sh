#!/bin/sh
set -eu

status=0

for workflow in .github/workflows/*.yml .github/workflows/*.yaml; do
    [ -e "$workflow" ] || continue
    if ! awk '
        /uses:[[:space:]]*/ {
            action = $0
            sub(/^.*uses:[[:space:]]*/, "", action)
            split(action, fields, /[[:space:]#]/)
            reference = fields[1]
            sub(/^.*@/, "", reference)
            if (reference !~ /^[0-9a-f]{40}$/) {
                printf "%s:%d: unpinned action: %s\n", FILENAME, FNR, fields[1]
                invalid = 1
            }
        }
        END { exit invalid }
    ' "$workflow"; then
        status=1
    fi
done

exit "$status"
