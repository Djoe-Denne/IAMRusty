# syntax=docker/dockerfile:1
# Linux delivery only; the existing image toolchain supplies selected artifacts.
ARG BUILD_IMAGE=local/build-artifacts:latest
ARG RUNTIME_IMAGE=debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
FROM ${BUILD_IMAGE} AS build-source
ARG SOURCE_REVISION=unattested
ARG SDK_REVISION=unattested
RUN if [ "$SOURCE_REVISION" != unattested ]; then \
    case "$(cat /app/target/release/delivery-profile)" in images|workspace) ;; *) exit 1 ;; esac \
    && test "$(cat /app/target/release/source-revision)" = "$SOURCE_REVISION" \
    && test "$(cat /app/target/release/sdk-revision)" = "$SDK_REVISION"; fi

FROM ${RUNTIME_IMAGE} AS runtime
ARG RUNTIME_OPENSSL_PACKAGE=libssl3
ARG SOURCE_REVISION=unattested
ARG SDK_REVISION=unattested
LABEL org.opencontainers.image.source="https://github.com/Djoe-Denne/AIForAll" \
      org.opencontainers.image.revision=$SOURCE_REVISION aiforall.dev.sdk-revision=$SDK_REVISION
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates ${RUNTIME_OPENSSL_PACKAGE} curl \
    && rm -rf /var/lib/apt/lists/* \
    && (getent group 1000 >/dev/null || groupadd --gid 1000 appuser) \
    && (getent passwd 1000 >/dev/null || useradd --uid 1000 --gid 1000 --no-create-home appuser) \
    && mkdir /app/config
COPY --from=build-source /app/target/release/oauth-connect-service /app/oauth-connect-service
USER 1000:1000
EXPOSE 8080 8443
ENTRYPOINT ["/app/oauth-connect-service"]
