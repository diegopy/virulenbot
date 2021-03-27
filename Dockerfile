FROM rust:1.51 AS build
WORKDIR /app
RUN apt-get update && apt-get install -y musl-dev musl-tools

# Download the target for static linking.
RUN rustup target add x86_64-unknown-linux-musl

# Create a dummy project and build the app's dependencies.
# If the Cargo.toml or Cargo.lock files have not changed,
# we can use the docker build cache and skip these (typically slow) steps.
RUN cargo new --bin virulenbot
WORKDIR /app/virulenbot
COPY Cargo.toml Cargo.lock ./
RUN cargo build --target x86_64-unknown-linux-musl --release
RUN rm src/*.rs

# Copy the source and build the application.
COPY src ./src
RUN cargo build --target x86_64-unknown-linux-musl --release
RUN strip target/x86_64-unknown-linux-musl/release/virulenbot

# Copy the statically-linked binary into a scratch container.
FROM scratch
COPY --from=build /app/virulenbot/target/x86_64-unknown-linux-musl/release/virulenbot .
USER 1000
CMD ["./virulenbot"]
