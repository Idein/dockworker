# iostream-docker-test
#

FROM debian:jessie-20180831

WORKDIR /test

COPY entrypoint.sh /test/entrypoint.sh
COPY ./sample      /test/sample

ENTRYPOINT ["./entrypoint.sh"]

