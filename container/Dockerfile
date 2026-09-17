# The SDK tag is mutable, so the default is the digest verified for this spike.
# An explicit --build-arg BASE_IMAGE=... is supported for controlled upgrades.
ARG BASE_IMAGE=xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba
FROM ${BASE_IMAGE}

USER root

ARG RUST_VERSION=1.85.1
ARG RUST_HOST=x86_64-unknown-linux-gnu
ARG RUST_ARCHIVE_SHA256=912c76f602f5f630138fca2388b2688c2c1aabcf823d71890b2b9155f743792e
ARG RUST_STD_SHA256=a33639b80353077a37b4e313d0882b074fa30e8fe968c6d6d36334fc74b3a845

# Install immutable Rust distribution archives directly. This avoids a moving
# rustup bootstrap binary while retaining the exact target standard library.
RUN set -eux; \
    rust_archive="rust-${RUST_VERSION}-${RUST_HOST}.tar.xz"; \
    std_archive="rust-std-${RUST_VERSION}-thumbv7em-none-eabihf.tar.xz"; \
    curl -fsSLO "https://static.rust-lang.org/dist/${rust_archive}"; \
    echo "${RUST_ARCHIVE_SHA256}  ${rust_archive}" | sha256sum -c -; \
    curl -fsSLO "https://static.rust-lang.org/dist/${std_archive}"; \
    echo "${RUST_STD_SHA256}  ${std_archive}" | sha256sum -c -; \
    tar -xJf "${rust_archive}"; \
    "rust-${RUST_VERSION}-${RUST_HOST}/install.sh" --prefix=/opt/rust --disable-ldconfig; \
    tar -xJf "${std_archive}"; \
    "rust-std-${RUST_VERSION}-thumbv7em-none-eabihf/install.sh" --prefix=/opt/rust --disable-ldconfig; \
    rm -rf "${rust_archive}" "${std_archive}" \
      "rust-${RUST_VERSION}-${RUST_HOST}" \
      "rust-std-${RUST_VERSION}-thumbv7em-none-eabihf"; \
    /opt/rust/bin/rustc --version --verbose; \
    /opt/rust/bin/rustc --print target-libdir --target thumbv7em-none-eabihf

ENV PATH=/opt/rust/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    CARGO_HOME=/opt/cargo

RUN mkdir -p /opt/cargo && chmod 0777 /opt/cargo

USER user
WORKDIR /workspace
ENTRYPOINT []
CMD ["/bin/bash"]
