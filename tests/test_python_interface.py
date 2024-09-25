#!/usr/bin/env python
import pathlib
import re

import pytest

import rustfluent as fluent


data_dir = pathlib.Path(__file__).parent.resolve() / "data"

# Bidirectional markers.
# See https://unicode.org/reports/tr9/#Directional_Formatting_Characters
BIDI_OPEN, BIDI_CLOSE = "\u2068", "\u2069"


def test_en_basic():
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_basic_with_named_arguments():
    bundle = fluent.Bundle(
        language="en",
        ftl_filenames=[str(data_dir / "en.ftl")],
    )
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_with_args():
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == f"Hello, {BIDI_OPEN}Bob{BIDI_CLOSE}"
    )


@pytest.mark.parametrize(
    "description, identifier, variables, expected",
    (
        ("String", "hello-user", {"user": "Bob"}, f"Hello, {BIDI_OPEN}Bob{BIDI_CLOSE}"),
        ("Integer", "apples", {"numberOfApples": 10}, f"{BIDI_OPEN}10{BIDI_CLOSE} apples"),
    ),
)
def test_variables_of_different_types(description, identifier, variables, expected):
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])

    result = bundle.get_translation(identifier, variables=variables)

    assert result == expected


@pytest.mark.parametrize(
    "key",
    (
        object(),
        34.3,
        10,
    ),
)
def test_invalid_variable_keys_raise_type_error(key):
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])

    with pytest.raises(TypeError, match="Variable key not a str, got"):
        bundle.get_translation("hello-user", variables={key: "Bob"})


@pytest.mark.parametrize(
    "value",
    (
        object(),
        34.3,
        1_000_000_000_000,  # Larger than signed long integer.
    ),
)
def test_invalid_variable_values_use_key_instead(value):
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])

    result = bundle.get_translation("hello-user", variables={"user": value})

    assert result == f"Hello, {BIDI_OPEN}user{BIDI_CLOSE}"


def test_fr_basic():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert bundle.get_translation("hello-world") == "Bonjour le monde!"


def test_fr_with_args():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == f"Bonjour, {BIDI_OPEN}Bob{BIDI_CLOSE}!"
    )


@pytest.mark.parametrize(
    "number, expected",
    (
        (1, "One"),
        (2, "Something else"),
        # Note that for selection to work, the variable must be an integer.
        # So "1" is not equivalent to 1.
        ("1", "Something else"),
    ),
)
def test_selector(number, expected):
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])

    result = bundle.get_translation("with-selector", variables={"number": number})

    assert result == expected


def test_new_overwrites_old():
    bundle = fluent.Bundle(
        "en",
        [str(data_dir / "fr.ftl"), str(data_dir / "en_hello.ftl")],
    )
    assert bundle.get_translation("hello-world") == "Hello World"
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == f"Bonjour, {BIDI_OPEN}Bob{BIDI_CLOSE}!"
    )


def test_id_not_found():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    with pytest.raises(ValueError):
        bundle.get_translation("missing", variables={"user": "Bob"})


def test_file_not_found():
    with pytest.raises(FileNotFoundError):
        fluent.Bundle("fr", [str(data_dir / "none.ftl")])


@pytest.mark.parametrize("pass_strict_argument_explicitly", (True, False))
def test_parses_other_parts_of_file_that_contains_errors_in_non_strict_mode(
    pass_strict_argument_explicitly,
):
    kwargs = dict(strict=False) if pass_strict_argument_explicitly else {}

    bundle = fluent.Bundle("fr", [str(data_dir / "errors.ftl")], **kwargs)
    translation = bundle.get_translation("valid-message")

    assert translation == "I'm valid."


def test_raises_parser_error_on_file_that_contains_errors_in_strict_mode():
    filename = str(data_dir / "errors.ftl")
    with pytest.raises(fluent.ParserError, match=re.escape(f"Error when parsing {filename}.")):
        fluent.Bundle("fr", [filename], strict=True)
