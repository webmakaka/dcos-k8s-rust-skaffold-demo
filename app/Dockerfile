FROM rust:1.57 as builder

RUN apt-get update -y && apt upgrade -y

WORKDIR /build

COPY ./ ./

RUN cargo build --release


FROM rust:1.57 as runner

RUN apt update -y && apt upgrade -y
RUN apt-get install -y libpq-dev iputils-ping

WORKDIR /app

COPY --from=builder /build/target/release/rust-web-demo .

CMD ["./rust-web-demo"]
