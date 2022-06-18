SUBMODULES = helgi kara sigrun ymir

.PHONY: build
build: # Build all submodules
	for module in $(SUBMODULES); do \
		make build -C ./$${module}; \
	done

.PHONY: lint
lint: # Lint all submodules
	for module in $(SUBMODULES); do \
		make lint -C ./$${module}; \
	done

.PHONY: clean
clean: # Lint all submodules
	for module in $(SUBMODULES); do \
		make clean -C ./$${module}; \
	done

.PHONY: test
test: # Lint all submodules
	for module in $(SUBMODULES); do \
		make test -C ./$${module}; \
	done
