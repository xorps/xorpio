.PHONY: basic
basic:
	docker build --build-arg TARGET=x86_64-unknown-none -f Dockerfile.examples . --platform linux/amd64 -t basic

.PHONY: run
run:
	docker run --rm -it -p 8080:8080 basic