# Build stage
# Choose base image
# Use cargo-chef crate to speed up the building process
FROM lukemathwalker/cargo-chef:latest-rust-1.74.0 as chef

# Set up working directory
WORKDIR /app
# Install system dependencies
RUN apt update && apt install lld clang -y
# Copy all file into the working directory

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
# Use prepared offline database schema and query data.
ENV SQLX_OFFLINE true
# Build the binary
RUN cargo build --release --bin zero2prod


# Runtime stage
FROM rust:1.74.0 AS Runtime

WORKDIR /app
# Copy the compiled binary from the builder environment
# to the runtime environment
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration 
# Specify env variables needed at runtime.
ENV APP_ENV production
# Choose a docker entry point for
ENTRYPOINT [ "./zero2prod" ]
