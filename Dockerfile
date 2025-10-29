FROM rust:latest

# Install required system libraries
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config

RUN useradd -ms /bin/bash duser
USER duser
# Copy and build daemon
WORKDIR /home/duser/rust_daemon
COPY . .
RUN cargo build --release
# Build CLI
RUN cd /home/duser/rust_daemon/cli
RUN cargo build --release

#CMD ["./target/release/rust_daemon"]
