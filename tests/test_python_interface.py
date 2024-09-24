#!/usr/bin/env python
import pathlib
import re

import pytest

import rustfluent as fluent


data_dir = pathlib.Path(__file__).parent.resolve() / "data"


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
        bundle.get_translation("hello-user", variables={"user": "Bob"}) == "Hello, \u2068Bob\u2069"
    )


def test_fr_basic():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert bundle.get_translation("hello-world") == "Bonjour le monde!"


def test_fr_with_args():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == "Bonjour, \u2068Bob\u2069!"
    )


@pytest.mark.parametrize(
    "number, expected",
    (
        pytest.param(1, "One", marks=pytest.mark.xfail),
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
        == "Bonjour, \u2068Bob\u2069!"
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
