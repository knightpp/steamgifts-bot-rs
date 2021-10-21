FROM docker.io/archlinux:latest as build
RUN pacman -Syu --noconfirm rustup musl gcc
RUN rustup update stable
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
ADD Cargo.toml .
ADD Cargo.lock .
RUN cargo fetch
ADD src src
RUN cargo install --path . --root /app --target x86_64-unknown-linux-musl

FROM docker.io/alpine:latest
COPY --from=build "/app/bin/steamgiftsbot" "/bin/steamgiftsbot"
CMD ["/bin/steamgiftsbot"]