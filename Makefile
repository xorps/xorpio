EXAMPLE ?= http
ARCH ?= amd64
IMAGE := xorpio-$(EXAMPLE):$(ARCH)
# Docker's default seccomp profile blocks io_uring.
RUN_FLAGS := $(if $(filter uring,$(EXAMPLE)),--security-opt seccomp=unconfined)
TARGETS := x86_64-unknown-none aarch64-unknown-none

.PHONY: check test image run tables

check:
	cargo fmt --check
	$(foreach t,$(TARGETS),cargo clippy --target $(t) --lib --examples -- -D warnings &&) true

# Integration tests need a Linux host; run them in Docker on both arches.
# seccomp=unconfined: the default Docker profile blocks io_uring.
test:
	$(foreach a,amd64 arm64,docker run --rm --platform linux/$(a) -v "$(CURDIR)":/src:ro \
		--security-opt seccomp=unconfined -e CARGO_TARGET_DIR=/tmp/target -w /src rust:1.99.1-slim-bookworm \
		cargo test --locked --tests &&) true

image:
	docker build --platform linux/$(ARCH) --build-arg EXAMPLE=$(EXAMPLE) -f Dockerfile.examples -t $(IMAGE) .

run: image
	docker run --rm -it --platform linux/$(ARCH) $(RUN_FLAGS) -p 8080:8080 $(IMAGE)

# Regenerate src/sys/nr_*.rs and src/errno.rs from the uapi headers of
# Debian's linux-libc-dev, dumped natively on each architecture.
TABLES_IMAGE := debian:trixie-slim
DUMP = printf '\043include <%s>\n' $(1) | gcc -dM -E - > /out/$(2)_$(a).txt
tables:
	mkdir -p target/tables
	$(foreach a,amd64 arm64,docker run --rm --platform linux/$(a) \
		-v "$(CURDIR)/target/tables":/out $(TABLES_IMAGE) sh -c \
		"apt-get update -qq && apt-get install -y -qq gcc linux-libc-dev >/dev/null && \
		$(call DUMP,asm/unistd.h,nr) && $(call DUMP,errno.h,errno) && \
		dpkg-query -W -f '\$${source:Upstream-Version}' linux-libc-dev > /out/version" &&) true
	python3 tools/gen_tables.py target/tables "$$(cut -d. -f1-2 target/tables/version)"
	cargo fmt
