FROM docker.io/rust:latest
WORKDIR /app
ADD Cargo.toml .
ADD Cargo.lock .
RUN cargo fetch
ADD src src

RUN cargo install --path . --root /app

CMD ["/app/bin/steamgiftsbot"]
