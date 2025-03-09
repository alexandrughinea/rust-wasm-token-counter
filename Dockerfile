# Build stage
FROM rust:1.75-slim AS builder

# Install minimal dependencies
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh && \
    rustup target add wasm32-unknown-unknown && \
    rustup default stable

WORKDIR /build

COPY . .

RUN rustc --version && \
    cargo --version && \
    wasm-pack --version && \
    RUST_BACKTRACE=1 wasm-pack build --target web --dev || \
    (echo "Build failed. Showing debug information:" && \
     ls -la && \
     cat target/wasm32-unknown-unknown/debug/.cargo-lock && \
     exit 1)


RUN mkdir -p dist && \
    cp -r pkg dist/ && \
    cp public/index.html dist/ && \
    cp public/worker.js dist/

# Production stage
FROM nginx:alpine

# Copy built files
COPY --from=builder /build/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
