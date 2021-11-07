FROM arm32v7/debian:bullseye-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && update-ca-certificates \
 && apt-get clean autoclean \
 && apt-get autoremove --yes \
 && rm -rf /var/lib/{apt,dpkg,cache,log}/

WORKDIR /app

RUN mkdir -p /app/src

COPY ./tmp/dns ./
COPY ./static ./static
COPY ./blu.txt ./

EXPOSE 53/udp
ENTRYPOINT ["./dns"]
