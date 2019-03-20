#!/bin/bash

set -ex

cd .docker

docker build --build-arg USERID=$(id -u) -t ember-shorty .
