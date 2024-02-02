# display trapped signal
#

FROM debian:latest

WORKDIR /test

COPY entrypoint.sh /test/entrypoint.sh

STOPSIGNAL 37

ENTRYPOINT ["./entrypoint.sh"]

