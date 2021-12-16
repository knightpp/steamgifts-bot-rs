FROM docker.io/rust:1.56.0-alpine as build
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-alpine-linux-musl-gcc \
	CC_x86_64_unknown_linux_musl=x86_64-alpine-linux-musl-gcc \
	CXX_x86_64_unknown_linux_musl=x86_64-alpine-linux-musl-g++
RUN apk add --no-cache musl-dev protobuf-dev

WORKDIR /app
COPY Cargo.toml .
COPY Cargo.lock .
RUN cargo fetch
COPY src src

RUN cargo install --path . --root /app --target x86_64-unknown-linux-musl

FROM docker.io/alpine:3.15
RUN apk add --no-cache curl
COPY --from=build "/app/bin/steamgiftsbot" "/bin/steamgiftsbot"
CMD ["/bin/steamgiftsbot"]