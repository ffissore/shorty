#!/usr/bin/env bash

set -ex

docker build -t ffissore/shorty:$TRAVIS_TAG .
docker tag ffissore/shorty:$TRAVIS_TAG ffissore/shorty:latest

echo "$DOCKER_PASSWORD" | docker login -u "$DOCKER_USERNAME" --password-stdin

docker push ffissore/shorty:$TRAVIS_TAG
docker push ffissore/shorty:latest
