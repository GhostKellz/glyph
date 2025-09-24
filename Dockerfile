FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/glyph /usr/local/bin/glyph
EXPOSE 7331
CMD ["glyph", "serve", "--address", "0.0.0.0:7331"]