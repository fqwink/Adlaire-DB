DOCKER ?= docker
COMPOSE_FILE ?= docker-compose.test.yml

.PHONY: test-docker fmt-docker docker-build docker-shell

test-docker:
	./scripts/test-docker.sh

fmt-docker:
	./scripts/test-docker.sh cargo fmt --all -- --check

docker-build:
	$(DOCKER) compose -f $(COMPOSE_FILE) build test

docker-shell:
	$(DOCKER) compose -f $(COMPOSE_FILE) run --rm test bash
