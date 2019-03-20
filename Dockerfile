# build image

FROM alpine as build

RUN apk update && \
    apk upgrade && \
    apk add cargo rust

WORKDIR /app

ADD ./ /app

RUN cargo build --release -p shorty-http --target-dir target/alpine

#final image

FROM alpine

RUN apk update && \
    apk upgrade && \
    apk add libgcc

WORKDIR /app

COPY --from=build /app/target/alpine/release/shorty-http /app

ENV SHORTENER_HOST 0.0.0.0

CMD /app/shorty-http
