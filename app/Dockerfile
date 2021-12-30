FROM rust:1.57 as builder
# FROM rust:1.57.0-alpine3.14

# RUN apk update
RUN apt-get update -y && apt upgrade -y

# RUN apt-get install musl-tools -y

# RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

COPY ./ ./

RUN cargo build --release


# FROM alpine:3.15
# FROM rust:1.57.0-alpine3.14
FROM rust:1.57 as runner

# RUN apk add --no-cache libpq
RUN apt update -y && apt upgrade -y
RUN apt-get install -y libpq-dev iputils-ping

WORKDIR /app

COPY --from=builder /build/target/release/rust-web-demo .

CMD ["./rust-web-demo"]
