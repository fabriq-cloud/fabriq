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

WORKDIR /akira

COPY ./ .

RUN curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v3.19.4/protoc-3.19.4-linux-x86_64.zip"
RUN unzip protoc-3.19.4-linux-x86_64.zip -d $HOME/protoc
RUN mv $HOME/protoc/bin/protoc /usr/bin/protoc
RUN cargo build --release

#############################################################x#######################################
## Final image
####################################################################################################
FROM alpine:latest

# Import service user and group from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /akira

# Install glibc and libgcc
RUN apk upgrade --no-cache && apk add --no-cache gcompat libgcc

# Copy our build
COPY --from=builder /akira/target/release/service /akira/service

# Use the unprivileged service user during execution.
USER service::service

CMD ["./service"]
