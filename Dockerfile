FROM arm32v7/rust:latest

WORKDIR /app

RUN mkdir -p /app/src

COPY ./tmp/dns ./
COPY ./static ./static
COPY ./blu.txt ./

EXPOSE 53/udp
ENTRYPOINT ["./dns"]
