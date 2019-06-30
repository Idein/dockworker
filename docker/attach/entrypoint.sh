#!/bin/sh

usage() {
	echo "Usage: $0 stdout stderr" >&2
	exit 1
}

if [ $# -ne 2 ]; then
	usage
fi

# Wait for SIGUSR1 before continuing
# (http://mywiki.wooledge.org/SignalTrap#When_is_the_signal_handled.3F)
if [[ ! -z "${WAIT_BEFORE_CONTINUING}" ]]
then
	pid=
	trap '[[ $pid ]] && kill "$pid"' SIGUSR1
	sleep 10000 & pid=$!
	wait
	pid=
fi

stdout=$1
stderr=$2

dd bs=32 status=none if=$stdout of=/dev/stdout &
outp=$!
dd bs=32 status=none if=$stderr of=/dev/stderr &
errp=$!

wait $outp $errp

