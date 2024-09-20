#!/usr/bin/env python

import pathlib

import pytest

import rustfluent as fluent


data_dir = pathlib.Path(__file__).parent.resolve() / "data"


def test_en_basic():
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_with_args():
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])
    assert bundle.get_translation("hello-user", user="Bob") == "Hello, \u2068Bob\u2069"


def test_fr_basic():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert bundle.get_translation("hello-world") == "Bonjour le monde!"


def test_fr_with_args():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    assert bundle.get_translation("hello-user", user="Bob") == "Bonjour, \u2068Bob\u2069!"


def test_new_overwrites_old():
    bundle = fluent.Bundle(
        "en",
        [str(data_dir / "fr.ftl"), str(data_dir / "en_hello.ftl")],
    )
    assert bundle.get_translation("hello-world") == "Hello World"
    assert bundle.get_translation("hello-user", user="Bob") == "Bonjour, \u2068Bob\u2069!"


def test_id_not_found():
    bundle = fluent.Bundle("fr", [str(data_dir / "fr.ftl")])
    with pytest.raises(ValueError):
        bundle.get_translation("missing", user="Bob")


def test_file_not_found():
    with pytest.raises(FileNotFoundError):
        fluent.Bundle("fr", [str(data_dir / "none.ftl")])


def test_file_has_errors():
    with pytest.raises(ValueError):
        fluent.Bundle("fr", [str(data_dir / "errors.ftl")])
