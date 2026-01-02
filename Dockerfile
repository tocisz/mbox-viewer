# Stage 1: Build Frontend
FROM rust:alpine as frontend-builder

# Install dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev libc-dev nodejs npm binaryen

# Install trunk
RUN cargo install cargo-binstall
RUN cargo binstall trunk

WORKDIR /app
COPY . .

WORKDIR /app/frontend

# Install npm dependencies (Tailwind)
# Note: In some environments npx might need explicit install, but usually comes with npm
RUN npm install

# Build frontend
RUN rustup target add wasm32-unknown-unknown
RUN trunk build --release

# Stage 2: Build Backend
FROM rust:alpine as backend-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev libc-dev

WORKDIR /app
COPY . .

WORKDIR /app/backend

# Add musl target and build statically
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl --bin backend

# Stage 3: Runtime
FROM alpine:latest

# Install runtime deps if needed (usually none for static rust, but ca-certificates is good)
RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app

# Copy Backend Binary
COPY --from=backend-builder /app/backend/target/x86_64-unknown-linux-musl/release/backend /app/backend

# Copy Frontend Artifacts
COPY --from=frontend-builder /app/frontend/dist /app/frontend

# Copy Entrypoint
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Environment Variables
ENV PORT=8001
ENV HOST=0.0.0.0
ENV INDEX_PATH=/data/index
ENV ATTACHMENTS_DIR=/data/attachments
ENV FRONTEND_DIR=/app/frontend
ENV MBOX_FILE=/data/mail.mbox

EXPOSE 8001
VOLUME ["/data"]

ENTRYPOINT ["/app/entrypoint.sh"]
