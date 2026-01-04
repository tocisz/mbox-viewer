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
# RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --bin backend

# Stage 3: Runtime
# Stage 3: Runtime
FROM scratch

# Copy certificates
COPY --from=alpine:latest /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

WORKDIR /app

# Copy Backend Binary
COPY --from=backend-builder /app/backend/target/release/backend /app/backend

# Copy Frontend Artifacts (for locally built docker image, we usually want separate files if not embedded)
# BUT if we are building scratch we MUST usage embedded.
# The previous multi-stage build did NOT usage embed_frontend feature for backend!
# Let's fix that - local docker build MUST embed frontend to work in scratch easily.
# Alternatively we can COPY frontend dist folder, but ServeDir works fine in scratch? Yes.
COPY --from=frontend-builder /app/frontend/dist /app/frontend

# Environment Variables
ENV PORT=8001
ENV HOST=0.0.0.0
ENV INDEX_PATH=/data/index
ENV ATTACHMENTS_DIR=/data/attachments
ENV FRONTEND_DIR=/app/frontend
ENV MBOX_FILE=/data/mail.mbox

EXPOSE 8001
VOLUME ["/data"]

ENTRYPOINT ["/app/backend", "serve"]
