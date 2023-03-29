### Indexer GRPC Image ###

FROM debian-base AS indexer-grpc

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \   
    apt-get update && apt-get install --no-install-recommends -y \    
        libssl1.1 \
        ca-certificates \
        net-tools \
        tcpdump \
        iproute2 \
        netcat \
        libpq-dev \
        curl

COPY --link --from=tools-builder /aptos/dist/aptos-indexer-grpc-cache-worker /usr/local/bin/aptos-indexer-grpc-cache-worker
COPY --link --from=tools-builder /aptos/dist/aptos-indexer-grpc-file-store /usr/local/bin/aptos-indexer-grpc-file-store
COPY --link --from=tools-builder /aptos/dist/aptos-indexer-grpc-data-service /usr/local/bin/aptos-indexer-grpc-data-service
COPY --link --from=tools-builder /aptos/dist/aptos-indexer-grpc-parser /usr/local/bin/aptos-indexer-grpc-parser

# The health check port
EXPOSE 8080
# The gRPC port
EXPOSE 50051

ENV RUST_LOG_FORMAT=json

# add build info
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}