# display trapped signal
#

FROM debian:12.4

WORKDIR /test

COPY entrypoint.sh /test/entrypoint.sh

STOPSIGNAL 37

ENTRYPOINT ["./entrypoint.sh"]

