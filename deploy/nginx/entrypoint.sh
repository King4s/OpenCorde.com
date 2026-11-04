#!/bin/sh
set -eu
: "${BACKEND_IP:?BACKEND_IP is required}"
: "${MEDIA_IP:?MEDIA_IP is required}"
: "${TURN_DOMAIN:?TURN_DOMAIN is required}"
# An explicit allowlist preserves nginx variables such as $remote_addr.
envsubst '${BACKEND_IP} ${MEDIA_IP} ${TURN_DOMAIN}' \
    < /etc/opencorde/nginx.conf.template > /tmp/opencorde-nginx.conf
exec "$@"
