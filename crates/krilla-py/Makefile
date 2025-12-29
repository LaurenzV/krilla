.PHONY: docs
docs:
	@echo "Building Python extension..."
	@uv run maturin develop
	@echo "Building documentation..."
	@uv run sphinx-build docs/source docs/build/html
	@echo "Documentation built at docs/build/html/index.html"

.PHONY: docs-serve
docs-serve: docs
	@echo "Serving documentation at http://localhost:8000"
	@uv run python -m http.server -d docs/build/html 8000

.PHONY: docs-clean
docs-clean:
	@echo "Cleaning documentation build directory..."
	@rm -rf docs/build
	@echo "Documentation build directory cleaned"

.PHONY: check
check:
	@echo "Running Python formatting check..."
	@uv run ruff format
	@echo "Running Python linting..."
	@uv run ruff check --fix
	@echo "Running Rust formatting check..."
	@cargo fmt
	@echo "Running Rust clippy..."
	@cargo clippy
	@echo "Running Rust check..."
	@cargo check
	@echo "Running Rust tests..."
	@cargo test
	@echo "Running Python tests..."
	@uv run pytest
	@echo "All checks passed!"

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  check        - Run all checks (formatting, linting, tests) for Rust and Python"
	@echo "  docs         - Build the documentation"
	@echo "  docs-serve   - Build and serve the documentation at http://localhost:8000"
	@echo "  docs-clean   - Clean the documentation build directory"
	@echo "  help         - Show this help message"
