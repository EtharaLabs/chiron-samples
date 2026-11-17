#!/bin/bash
# Retry a Reward Kit pass that fails, such as a judge call the endpoint refused.
for attempt in 1 2 3; do
  /opt/rewardkit/bin/rewardkit "$@" && exit 0
  status=$?
  [ "$attempt" -lt 3 ] && sleep $((attempt * 60))
done
exit "$status"
