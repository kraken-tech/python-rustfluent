# rustfluent

A Python interface to the Rust Fluent Library.

This project is a small shim around [fluent-rs](https://github.com/projectfluent/fluent-rs), so it
can be used from Python.

> [!WARNING]
> This package is under active development, and breaking changes may be released at any time. Be sure to pin to
> specific versions if you're using this package in a production environment.

## Prerequisites

This package supports:

- Python 3.11
- Python 3.12

## Installation

```
pip install rustfluent
```

## Usage

```python
import rustfluent

# First load a bundle
bundle = rustfluent.Bundle(
    "en",
    [
        # Multiple FTL files can be specified. Entries in later
        # files overwrite earlier ones.
        "en.ftl",
    ],
)

# Fetch a translation
assert bundle.get_translation("hello-world") == "Hello World"

# Fetch a translation that includes variables
assert bundle.get_translation("hello-user", variables={"user": "Bob"}) == "Hello, \u2068Bob\u2069"
```

The Unicode characters around "Bob" in the above example are for
[Unicode bidirectional handling](https://www.unicode.org/reports/tr9/).

## API reference

### `Bundle` class

A set of translations for a specific language.

```python
import rustfluent

bundle = rustfluent.Bundle(
    language="en-US",
    ftl_files=[
        "/path/to/messages.ftl",
        "/path/to/more/messages.ftl",
    ],
)
```

#### Parameters

| Name        | Type             | Description                                                                                                                                                              |
|-------------|------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `language`  | `str`            | [Unicode Language Identifier](https://unicode.org/reports/tr35/tr35.html#Unicode_language_identifier) for the language.                                                  |
| `ftl_files` | `list[str]`      | Full paths to the FTL files containing the translations. Entries in later files overwrite earlier ones.                                                                  |
| `strict`    | `bool`, optional | In strict mode, a `ParserError` will be raised if there are any errors in the file. In non-strict mode, invalid Fluent messages will be excluded from the Bundle. |

#### Raises

- `FileNotFoundError` if any of the FTL files could not be found.
- `rustfluent.ParserError` if any of the FTL files contain errors (strict mode only).

### `Bundle.get_translation`

```
>>> bundle.get_translation(identifier="hello-world")
"Hello, world!"
>>> bundle.get_translation(identifier="hello-user", variables={"user": "Bob"})
"Hello, Bob!"
```

#### Parameters

| Name         | Type                         | Description                            |
|--------------|------------------------------|----------------------------------------|
| `identifier` | `str`                        | The identifier for the Fluent message. |
| `variables`  | `dict[str, str | int ]`, optional                  | Any [variables](https://projectfluent.org/fluent/guide/variables.html) to be passed to the Fluent message. |

#### Return value

`str`: the translated message.

If there is a problem with a passed variable (e.g. it is of the wrong type or an integer that is larger than a
signed long integer), then the name of the variable will be used instead.

#### Raises

- `ValueError` if the message could not be found or has no translation available.
- `TypeError` if a passed variable name (i.e. a key in the `variables` dict) is not a string.

## Contributing

See [Contributing](./CONTRIBUTING.md).
