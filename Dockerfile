FROM arm32v7/rust:latest

WORKDIR /app

RUN mkdir -p /app/src
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
COPY ./Cargo.toml ./
COPY ./Cargo.lock ./
COPY ./benches ./benches
RUN cargo build --release

COPY . /app
RUN cargo build --release

EXPOSE 53/udp
ENTRYPOINT ["./target/release/dns", "-s"]