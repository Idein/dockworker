# iostream-docker-test
#

FROM alpine:3.8

WORKDIR /test

COPY entrypoint.sh /test/entrypoint.sh
COPY ./sample      /test/sample

ENTRYPOINT ["./entrypoint.sh"]

