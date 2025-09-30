# Multi-stage build for svg2gcode web application

# Stage 1: Build the WASM application
FROM rust:1.84 AS builder

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Install trunk from binary (faster than cargo install)
RUN wget -qO- https://github.com/trunk-rs/trunk/releases/download/v0.21.5/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf- && \
    mv trunk /usr/local/bin/

WORKDIR /app

# Copy workspace files
COPY Cargo.toml .
COPY lib ./lib
COPY cli ./cli
COPY web ./web

# Build the web application
WORKDIR /app/web
RUN trunk build --release

# Stage 2: Serve with nginx
FROM nginx:alpine

# Copy built files from builder
COPY --from=builder /app/web/dist /usr/share/nginx/html

# Copy nginx configuration
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # Enable gzip compression for WASM files
    gzip on;
    gzip_types application/wasm application/javascript text/css;
    gzip_min_length 1000;
}
EOF

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]