#!/bin/bash

docker run \
  -t -i \
  --rm \
  -p 4200:4200 \
  -p 35729:35729 \
  -v $(pwd):/app \
  -v ~/.npm:/home/username/.npm \
  ember-shorty \
  /usr/local/bin/ember server
