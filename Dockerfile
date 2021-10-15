FROM rust:latest as build
WORKDIR /app
ADD Cargo.toml .
ADD Cargo.lock .
RUN cargo fetch
ADD src src
# COPY . .
RUN cargo install --path . --root /app

FROM debian:latest
COPY --from=build /app/bin/steamgiftsbot .

CMD ["/steamgiftsbot"]
