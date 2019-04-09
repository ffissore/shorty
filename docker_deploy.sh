#!/usr/bin/env bash

set -ex

docker build -t ffissore/shorty:$TRAVIS_TAG .

echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

docker push ffissore/shorty
