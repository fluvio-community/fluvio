PYTHON ?= python3

.PHONY: test

verify:
	$(PYTHON) -m pytest tests
