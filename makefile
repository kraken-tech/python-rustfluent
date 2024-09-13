SHELL=/bin/bash

# If we're running in CI then store Pytest output in a format which CircleCI can parse
ifdef CIRCLECI
MYPY_ARGS=--junit-xml=test-results/mypy.xml
endif

# Standard entry points
# =====================

.PHONY:dev
dev: install_python_packages .git/hooks/pre-commit

.PHONY:test
test:
	cargo test
	pytest

.PHONY:matrix_test
matrix_test:
	nox

.PHONY:lint
lint: ruff_format ruff_lint cargo_format mypy

.PHONY:ruff_format
ruff_format:
	ruff format --check .

.PHONY:cargo_format
cargo_format:
	cargo fmt --check

.PHONY:ruff_lint
ruff_lint:
	ruff check .

.PHONY:mypy
mypy:
	mypy $(MYPY_ARGS)

.PHONY:format
format:
	ruff format .
	ruff check --fix .
	cargo fmt

.PHONY:update
update:
	uv pip compile pyproject.toml -q --upgrade --extra=dev --output-file=requirements/development.txt

.PHONY:package
package:
	python -m build

.PHONY:version_major
version_major:
	bump-my-version bump major

.PHONY:version_minor
version_minor:
	bump-my-version bump minor

.PHONY:version_patch
version_patch:
	bump-my-version bump patch


# Implementation details
# ======================

# Pip install all required Python packages
.PHONY:install_python_packages
install_python_packages: requirements/development.txt
	uv pip sync requirements/development.txt requirements/firstparty.txt

# Add new dependencies to requirements/development.txt whenever pyproject.toml changes
requirements/development.txt: pyproject.toml
	uv pip compile pyproject.toml -q --extra=dev --output-file=requirements/development.txt

.git/hooks/pre-commit:
	@if type pre-commit >/dev/null 2>&1; then \
		pre-commit install; \
	else \
		echo "WARNING: pre-commit not installed." > /dev/stderr; \
	fi
