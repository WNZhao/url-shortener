FROM rust:1.94 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs && cargo build --release && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/url-shortener /usr/local/bin/url-shortener

EXPOSE 3000
CMD ["url-shortener"]
