# Choose base image
FROM rust:1.74.0

# Set up working directory
WORKDIR /app

# Install system dependencies
RUN apt update && apt install lld clang -y

# Copy all file into the working directory
COPY . .

# Use prepared offline database schema and query data.
ENV SQLX_OFFLINE true
ENV APP_ENV production
# Build the binary
RUN cargo build --release

# Choose a docker entry point for
ENTRYPOINT [ "./target/release/zero2prod" ]
