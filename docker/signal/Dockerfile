# display trapped signal
#

FROM debian:jessie-20180831

WORKDIR /test

COPY entrypoint.sh /test/entrypoint.sh

STOPSIGNAL 37

ENTRYPOINT ["./entrypoint.sh"]

