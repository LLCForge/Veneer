# syntax=docker/dockerfile:1
# Playwright + Veneer test environment, built from this repository.
#   docker build -f docker/Dockerfile -t veneer-playwright .
#   docker run --rm --init --ipc=host -v "$PWD/tests:/app/tests" veneer-playwright npx playwright test

# --- Stage 1: WASM build of the npm package and the native CLI ---------------------------------------
FROM rust:1.94-slim-bookworm AS build
# Must equal the wasm-bindgen version in Cargo.lock.
ARG WASM_BINDGEN_VERSION=0.2.130
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install wasm-bindgen-cli --locked --version "${WASM_BINDGEN_VERSION}"
WORKDIR /src
COPY . .
RUN rustup target add wasm32-unknown-unknown \
 && cargo build -p veneer-wasm --target wasm32-unknown-unknown --release \
 && wasm-bindgen target/wasm32-unknown-unknown/release/veneer_wasm.wasm --target nodejs --out-dir packages/veneer/wasm \
 && cargo build -p veneer-cli --release

# --- Stage 2: Playwright image with browsers, Veneer packages and the CLI ---------------------------
FROM mcr.microsoft.com/playwright:v1.64.1-noble
ENV NODE_ENV=production
WORKDIR /app
RUN npm init -y >/dev/null \
 && npm install --no-audit --no-fund --save-exact playwright@1.64.1 @playwright/test@1.64.1 \
 && npm cache clean --force
COPY --from=build /src/packages/veneer /app/node_modules/@llcforge/veneer
COPY --from=build /src/packages/playwright-fixture /app/node_modules/@llcforge/veneer-playwright
COPY --from=build /src/packages/cdp /app/node_modules/@llcforge/veneer-cdp
COPY --from=build /src/target/release/veneer /usr/local/bin/veneer
COPY docker/playwright.config.js /app/playwright.config.js
RUN mkdir -p /app/tests && chown -R pwuser:pwuser /app
# The image ships an unprivileged user; browsers are already under PLAYWRIGHT_BROWSERS_PATH.
USER pwuser
CMD ["npx", "playwright", "test"]
