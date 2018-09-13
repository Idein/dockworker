#!/bin/sh

usage() {
	echo "Usage: $0 stdout stderr" >&2
	exit 1
}

if [ $# -ne 2 ]; then
	usage
fi

stdout=$1
stderr=$2

dd bs=32 status=none if=$stdout of=/dev/stdout &
outp=$!
dd bs=32 status=none if=$stderr of=/dev/stderr &
errp=$!

wait $outp $errp

