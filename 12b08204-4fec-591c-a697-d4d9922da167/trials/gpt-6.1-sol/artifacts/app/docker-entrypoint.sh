#!/bin/bash
set -e

/usr/sbin/nginx -c /app/docker/config/nginx-docker.conf &
nginx_pid=$!
./start_argus.sh &
backend_pid=$!

stop_services() {
    kill -TERM "$backend_pid" "$nginx_pid" 2>/dev/null || true
    wait "$backend_pid" "$nginx_pid" 2>/dev/null || true
}
trap stop_services EXIT
trap 'exit 0' TERM INT
wait -n "$backend_pid" "$nginx_pid"
