# Образ Selfhosted (спека 2026-10-12-delivery-variants-design, § 13). Сборка и запуск — README, раздел «Selfhosted».

# Фронт: статику берёт только сборка, поэтому без эмуляции под платформу образа.
FROM --platform=$BUILDPLATFORM node:22-slim AS front
WORKDIR /app
COPY app/package.json app/package-lock.json ./
# prepare ставит git-хуки lefthook, а .git в контексте сборки нет.
RUN npm ci --ignore-scripts
COPY app/ ./
# Версия релиза, как в release.yml: в тегах package.json хранит версию первого релиза, сервер пишет её в лог при старте.
ARG VERSION
RUN if [ -n "$VERSION" ]; then npm version "$VERSION" --no-git-tag-version --allow-same-version; fi
RUN VITE_DELIVERY=selfhosted npm run build

# Сервер: на Alpine Rust собирает под musl статически, статика фронта встраивается в бинарник.
# Версия Rust — та же, что в app/rust-toolchain.toml: при смене тулчейна менять в обоих местах.
FROM rust:1.94-alpine AS server
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY app/ ./
# main.rs берёт версию из package.json через include_str!.
COPY --from=front /app/package.json ./
COPY --from=front /app/dist dist/
RUN cargo build --release --locked -p longpole-server

FROM gcr.io/distroless/static:nonroot
COPY --from=server /app/target/release/longpole-server /longpole
EXPOSE 8080
ENTRYPOINT ["/longpole"]
