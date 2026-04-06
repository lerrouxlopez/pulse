FROM rust:1.78-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static
COPY Rocket.toml ./

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/pulse /app/pulse
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static
COPY --from=builder /app/Rocket.toml /app/Rocket.toml
RUN mkdir -p /app/data

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8001

EXPOSE 8001

ENTRYPOINT ["/app/pulse"]
