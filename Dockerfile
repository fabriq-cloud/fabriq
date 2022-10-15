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

RUN curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v3.21.7/protoc-3.21.7-linux-x86_64.zip"
RUN unzip protoc-3.21.7-linux-x86_64.zip -d $HOME/protoc
RUN mv $HOME/protoc/bin/protoc /usr/bin/protoc
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

# Install glibc and libgcc
RUN apk upgrade --no-cache && apk add --no-cache gcompat libgcc

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

# Install glibc and libgcc
RUN apk upgrade --no-cache && apk add --no-cache gcompat libgcc

# Copy our build
COPY --from=builder /fabriq/target/release/gitops /fabriq/gitops

# Use the unprivileged service user during execution.
USER service::service

CMD ["./gitops"]

#############################################################x#######################################
## Final reconciler container image
####################################################################################################
FROM alpine:latest AS reconciler

# Import service user and group from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /fabriq

# Install glibc and libgcc
RUN apk upgrade --no-cache && apk add --no-cache gcompat libgcc

# Copy our build
COPY --from=builder /fabriq/target/release/reconciler /fabriq/reconciler

# Use the unprivileged service user during execution.
USER service::service

CMD ["./reconciler"]