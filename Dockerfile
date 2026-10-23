FROM rust:1-slim-trixie AS build

RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown

# Trunk assemble l'application (wasm, CSS, fichiers statiques) ; il télécharge
# lui-même wasm-bindgen et wasm-opt.
ARG TRUNK_VERSION=0.22.0
ARG TRUNK_SHA256=f78c1989240c56592d8cfc4df5cc27399a3ff66a3f9cc819e16f1e3d809ce7c2
RUN curl -fsSL -o /tmp/trunk.tar.gz \
        "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz" \
    && echo "${TRUNK_SHA256}  /tmp/trunk.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/trunk.tar.gz -C /usr/local/bin trunk \
    && rm /tmp/trunk.tar.gz

WORKDIR /src

# tesseract.js et le modèle français (OCR de l'import photo, dans le navigateur).
COPY scripts ./scripts
COPY vendor ./vendor
RUN sh scripts/fetch-tesseract.sh

# Dépendances compilées dans une couche à part : seul un changement de
# Cargo.toml / Cargo.lock les fait recompiler.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked --target wasm32-unknown-unknown \
    && rm -rf src

COPY index.html Trunk.toml ./
COPY data ./data
COPY public ./public
COPY style ./style
COPY src ./src
RUN trunk build --release

# Caddy sert l'application et relaie /api vers Mealie (avec le token) et
# /ollama vers Ollama : pas de backend applicatif.
FROM caddy:2-alpine

COPY deploy/Caddyfile deploy/backends.caddy /etc/caddy/
COPY --from=build /src/dist /srv

EXPOSE 8000
