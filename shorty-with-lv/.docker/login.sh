#!/bin/bash

docker run \
  -t -i \
  --rm \
  -v $(pwd):/app \
  -v ~/.npm:/home/username/.npm \
  ember-shorty \
  bash
