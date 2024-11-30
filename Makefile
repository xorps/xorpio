.PHONY: basic
basic:
	docker build --build-arg TARGET=x86_64-unknown-none -f Dockerfile.examples . --platform linux/amd64 -t basic --no-cache --progress=plain
