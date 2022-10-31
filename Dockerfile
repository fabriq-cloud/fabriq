FROM rust:latest AS builder

ARG SERVICE_UID=10001

RUN apt update && apt upgrade -y
RUN apt install -y cmake

# unprivileged identity to run service as
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${SERVICE_UID}" \
    service

WORKDIR /fabriq

COPY ./ .

RUN curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v3.20.3/protoc-3.20.3-linux-x86_64.zip"
RUN unzip protoc-3.20.3-linux-x86_64.zip -d /fabriq/protoc
ENV PATH="${PATH}:/fabriq/protoc"
ENV PROTOC="/fabriq/protoc/bin/protoc"
ENV SQLX_OFFLINE=true

RUN cargo build --release

#############################################################x#######################################
## Final api container image
####################################################################################################
FROM alpine:latest AS api

# Import service user and group from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /fabriq

# Install glibc
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8
ENV GLIBC_REPO=https://github.com/sgerrand/alpine-pkg-glibc
ENV GLIBC_VERSION=2.35-r0

RUN set -ex && \
    apk --update add libstdc++ curl ca-certificates && \
    for pkg in glibc-${GLIBC_VERSION} glibc-bin-${GLIBC_VERSION}; \
    do curl -sSL ${GLIBC_REPO}/releases/download/${GLIBC_VERSION}/${pkg}.apk -o /tmp/${pkg}.apk; done && \
    apk add --allow-untrusted /tmp/*.apk && \
    rm -v /tmp/*.apk && \
    /usr/glibc-compat/sbin/ldconfig /lib /usr/glibc-compat/lib

# Copy our build
COPY --from=builder /fabriq/target/release/api /fabriq/api

# Use the unprivileged service user during execution.
USER service::service

CMD ["./api"]

#############################################################x#######################################
## Final gitops container image
####################################################################################################
FROM alpine:latest AS gitops

# Import service user and group from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /fabriq

# Install glibc
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8
ENV GLIBC_REPO=https://github.com/sgerrand/alpine-pkg-glibc
ENV GLIBC_VERSION=2.35-r0

RUN set -ex && \
    apk --update add libstdc++ curl ca-certificates && \
    for pkg in glibc-${GLIBC_VERSION} glibc-bin-${GLIBC_VERSION}; \
    do curl -sSL ${GLIBC_REPO}/releases/download/${GLIBC_VERSION}/${pkg}.apk -o /tmp/${pkg}.apk; done && \
    apk add --allow-untrusted /tmp/*.apk && \
    rm -v /tmp/*.apk && \
    /usr/glibc-compat/sbin/ldconfig /lib /usr/glibc-compat/lib

# Copy our build
COPY --from=builder /fabriq/target/release/gitops /fabriq/gitops

# Use the unprivileged service user during execution.
USER service::service

CMD ["./gitops"]
