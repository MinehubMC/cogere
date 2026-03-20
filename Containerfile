# https://github.com/LukeMathWalker/cargo-chef#running-the-binary-in-alpine
FROM clux/muslrust:nightly AS chef
USER root
RUN cargo install --locked cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin cogere

FROM alpine AS runtime
RUN addgroup -S cogere && adduser -S cogere -G cogere
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/cogere /usr/local/bin/
ENV COGERE_SOCKET_ADDR="0.0.0.0:9005"
ENV COGERE_DATA_FOLDER="/data"
ENV COGERE_PUBLIC_BASE_URL="http://localhost:9005"
VOLUME ["/data"]
EXPOSE 9005
USER cogere
CMD ["/usr/local/bin/cogere"]
