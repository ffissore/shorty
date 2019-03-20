FROM ffissore/docker-node-emberjs:latest

#by adding a user that matches the one used to start docker, we avoid file system permissions issues
ARG USERID
RUN adduser --disabled-login --gecos "" username --uid $USERID

USER $USERID
