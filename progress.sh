#!/bin/bash
# https://unix.stackexchange.com/a/666003

echo $1

shopt -s nullglob dotglob

nonempty=0
empty=0

echo "Gemeentes zonder URL's:"

for name in $1/*.txt; do
    if [ -f "$name" ] && [ ! -h "$name" ]; then
        # $name is a regular file

        if [ -s "$name" ]; then
            nonempty=$(( nonempty + 1 ))
        else
            empty=$(( empty + 1 ))
			echo $name
        fi
    fi
done

printf 'There were %d empty files and %d non-empty files\n' "$empty" "$nonempty"
