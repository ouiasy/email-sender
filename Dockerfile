FROM rust:1.87-slim-bookworm AS builder

WORKDIR /app

RUN apt update && apt install lld clang -y

COPY . .

ENV SQLX_OFFLINE true
ENV APP_ENVIRONMENT production
RUN cargo build --release

#---------------------------------------------

FROM ubuntu:latest AS runtime

WORKDIR /app
ENV APP_ENV production
#ENV APP_DATABASE__HOST 10.47.224.3
ENV APP_DATABASE__HOST /cloudsql/examples-463000:asia-northeast1:newsletter

RUN apt update -y \
    && apt install -y --no-install-recommends openssl ca-certificates \
    && apt autoremove -y \
    && apt clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/email_sender ./email_sender
COPY config ./config

ENTRYPOINT ["./email_sender"]