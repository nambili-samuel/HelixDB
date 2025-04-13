# Use single stage to keep all source and build tools
FROM rust:1.84

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang \
    llvm-dev \
    gcc \
    g++ \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set up working directory
WORKDIR /usr/src/app

# Copy source code
COPY . .

# Create non-root user
RUN useradd -m user && \
    chown -R user:user /usr/src/app
USER user

# Build command - this will run every time the container starts
CMD ["sh", "-c", "cd helix-container && RUSTFLAGS='' cargo build --release && ../target/release/helix-container"]