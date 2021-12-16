FROM rust:latest as build
WORKDIR /app
COPY Cargo.toml .
COPY Cargo.lock .
RUN cargo fetch
COPY src src
# COPY . .
RUN cargo install --path . --root /app

FROM debian:latest
COPY --from=build /app/bin/steamgiftsbot .

CMD ["/steamgiftsbot", "-d", "-t", "1h"]
