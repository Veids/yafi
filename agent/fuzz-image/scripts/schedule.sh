#!/usr/bin/env bash

_term(){
  kill -TERM "${child}"
}

trap _term SIGTERM

python /scripts/schedule.py &
child=$!
wait "${child}"
exit $?
