#!/usr/bin/env bash

set -e

properties='
client-api="pipewire-pulse"
application-name="aiat"
media.name="Desktop Audio"
media.role="production"
node.name="aiat"
'

pactl unload-module module-loopback || true
pactl load-module module-loopback source_output_properties="${properties}"
