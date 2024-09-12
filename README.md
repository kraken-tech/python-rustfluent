# rustfluent

A Python interface to the Rust Fluent Library.

This project is a small shim around [fluent-rs](https://github.com/projectfluent/fluent-rs), so it
can be used from Python.

## Prerequisites

This package supports:

- Python 3.10
- Python 3.11
- Python 3.12

## Installation

```
pip install rustfluent
```

## Usage

```python
import rustfluent as fluent

# First load a bundle
bundle = fluent.Bundle(
    "en",
    [
        # Multiple FTL files can be specified. Entries in later
        # files overwrite earlier ones.
        "en.ftl",
    ],
)

# Fetch a translation
assert bundle.get_translation("hello-world") == "Hello World"

# Fetch a translation that takes a keyword argument
assert bundle.get_translation("hello-user", user="Bob") == "Hello, \u2068Bob\u2069"
```

The Unicode characters around "Bob" in the above example are for
[Unicode bidirectional handling](https://www.unicode.org/reports/tr9/).

## Contributing

See [Contributing](./CONTRIBUTING.md).
