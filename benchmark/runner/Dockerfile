# Stage 1: Build writ binary from source
FROM rust:1.88-slim AS writ-builder
WORKDIR /writ
COPY Cargo.toml Cargo.lock ./
COPY writ-assembler/ writ-assembler/
COPY writ-cli/ writ-cli/
COPY writ-compiler/ writ-compiler/
COPY writ-dap/ writ-dap/
COPY writ-diagnostics/ writ-diagnostics/
COPY writ-golden/ writ-golden/
COPY writ-lsp/ writ-lsp/
COPY writ-module/ writ-module/
COPY writ-parser/ writ-parser/
COPY writ-runtime/ writ-runtime/
COPY writ-std/ writ-std/
RUN cargo build --release --bin writ

# Stage 2: Pre-compile Rust benchmark stubs
FROM rust:1.88-slim AS rust-bench-builder
WORKDIR /bench
COPY benchmark/cases/ ./cases/
RUN mkdir -p /bench/bin && \
    for rs in cases/*/*.rs; do \
        name=$(basename "$(dirname "$rs")"); \
        rustc -O -o "/bench/bin/${name}" "$rs"; \
    done

# Stage 3: Runtime image with all language runtimes
FROM ubuntu:24.04

# System packages: Lua 5.4, Python 3.12, jq, and Squirrel build dependencies
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    lua5.4 python3 jq \
    cmake build-essential git \
    curl ca-certificates gnupg && \
    rm -rf /var/lib/apt/lists/*

# Node.js 22 LTS via NodeSource (GPG key method, not pipe-to-bash)
RUN mkdir -p /etc/apt/keyrings && \
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" \
    > /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Squirrel 3.2 from source (squirrel3 is absent from Ubuntu 24.04 Noble apt repos)
# cmake produces binary 'sq' and shared libs libsquirrel.so / libsqstdlib.so
# Copy both binary and libraries, then run ldconfig so sq can find them at runtime
RUN git clone --depth 1 --branch v3.2 \
        https://github.com/albertodemichelis/squirrel.git /tmp/squirrel && \
    cd /tmp/squirrel && mkdir build && cd build && \
    cmake .. -DCMAKE_BUILD_TYPE=Release && make -j$(nproc) && \
    cp bin/sq /usr/local/bin/sq && \
    find /tmp/squirrel/build -name "*.so*" -exec cp {} /usr/local/lib/ \; && \
    ldconfig && \
    cd / && rm -rf /tmp/squirrel

# hyperfine 1.20.0 from GitHub Releases (not in Ubuntu apt repos)
RUN curl -sSL "https://github.com/sharkdp/hyperfine/releases/download/v1.20.0/hyperfine_1.20.0_amd64.deb" \
    -o /tmp/hyperfine.deb && \
    dpkg -i /tmp/hyperfine.deb && \
    rm /tmp/hyperfine.deb

# Clean up build dependencies to reduce image size
RUN apt-get purge -y cmake build-essential git && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

# Copy writ binary from builder stage
COPY --from=writ-builder /writ/target/release/writ /usr/local/bin/writ

# Copy pre-compiled Rust benchmark binaries
COPY --from=rust-bench-builder /bench/bin/ /bench/bin/

# Copy benchmark source files and runner script
COPY benchmark/cases/ /bench/cases/
COPY benchmark/runner/bench_runner.sh /bench/bench_runner.sh
RUN chmod +x /bench/bench_runner.sh

WORKDIR /bench
ENTRYPOINT ["/bench/bench_runner.sh"]
