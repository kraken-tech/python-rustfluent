#!/usr/bin/env python
import pathlib
from datetime import datetime

import pytest

import rustfluent as fluent


data_dir = pathlib.Path(__file__).parent.resolve() / "data"

# Bidirectional markers.
# See https://unicode.org/reports/tr9/#Directional_Formatting_Characters
BIDI_OPEN, BIDI_CLOSE = "\u2068", "\u2069"


def test_en_basic():
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_basic_str_path():
    bundle = fluent.Bundle("en", [str(data_dir / "en.ftl")])
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_basic_with_named_arguments():
    bundle = fluent.Bundle(
        language="en",
        ftl_filenames=[data_dir / "en.ftl"],
    )
    assert bundle.get_translation("hello-world") == "Hello World"


def test_en_with_variables():
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == f"Hello, {BIDI_OPEN}Bob{BIDI_CLOSE}"
    )


def test_en_with_variables_use_isolating_off():
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])
    assert (
        bundle.get_translation(
            "hello-user",
            variables={"user": "Bob"},
            use_isolating=False,
        )
        == "Hello, Bob"
    )


@pytest.mark.parametrize(
    "description, identifier, variables, expected",
    (
        ("String", "hello-user", {"user": "Bob"}, f"Hello, {BIDI_OPEN}Bob{BIDI_CLOSE}"),
        ("Integer", "apples", {"numberOfApples": 10}, f"{BIDI_OPEN}10{BIDI_CLOSE} apples"),
        (
            "Naive datetime",
            "date-message",
            {"date": datetime(2020, 1, 5)},
            f"The date is {BIDI_OPEN}2020-01-05{BIDI_CLOSE}.",
        ),
    ),
)
def test_variables_of_different_types(description, identifier, variables, expected):
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])

    result = bundle.get_translation(identifier, variables=variables)

    assert result == expected


def test_invalid_language():
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("$", [])

    assert str(exc_info.value) == "Invalid language: '$'"


@pytest.mark.parametrize(
    "key",
    (
        object(),
        34.3,
        10,
    ),
)
def test_invalid_variable_keys_raise_type_error(key):
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])

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
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])

    result = bundle.get_translation("hello-user", variables={"user": value})

    assert result == f"Hello, {BIDI_OPEN}user{BIDI_CLOSE}"


def test_fr_basic():
    bundle = fluent.Bundle("fr", [data_dir / "fr.ftl"])
    assert bundle.get_translation("hello-world") == "Bonjour le monde!"


def test_fr_with_args():
    bundle = fluent.Bundle("fr", [data_dir / "fr.ftl"])
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
    bundle = fluent.Bundle("en", [data_dir / "en.ftl"])

    result = bundle.get_translation("with-selector", variables={"number": number})

    assert result == expected


def test_new_overwrites_old():
    bundle = fluent.Bundle(
        "en",
        [data_dir / "fr.ftl", data_dir / "en_hello.ftl"],
    )
    assert bundle.get_translation("hello-world") == "Hello World"
    assert (
        bundle.get_translation("hello-user", variables={"user": "Bob"})
        == f"Bonjour, {BIDI_OPEN}Bob{BIDI_CLOSE}!"
    )


def test_id_not_found():
    bundle = fluent.Bundle("fr", [data_dir / "fr.ftl"])
    with pytest.raises(ValueError):
        bundle.get_translation("missing", variables={"user": "Bob"})


def test_file_not_found():
    with pytest.raises(FileNotFoundError):
        fluent.Bundle("fr", [data_dir / "none.ftl"])


@pytest.mark.parametrize("pass_strict_argument_explicitly", (True, False))
def test_parses_other_parts_of_file_that_contains_errors_in_non_strict_mode(
    pass_strict_argument_explicitly,
):
    kwargs = dict(strict=False) if pass_strict_argument_explicitly else {}

    bundle = fluent.Bundle("fr", [data_dir / "errors.ftl"], **kwargs)
    translation = bundle.get_translation("valid-message")

    assert translation == "I'm valid."


def test_raises_parser_error_on_file_that_contains_errors_in_strict_mode():
    filename = data_dir / "errors.ftl"

    with pytest.raises(fluent.ParserError) as exc_info:
        fluent.Bundle("fr", [filename], strict=True)

    message = str(exc_info.value)

    # Recombine first line if it was too long
    lines = message.split("\n")
    if lines[1].endswith(".ftl"):
        lines = [(lines[0] + lines[1]).replace("  │ ", ""), *lines[2:]]
    message = "\n".join(lines)
    # End recombination

    # Note: Line 2 has a trailing space after the pipe
    expected = '  × Found 1 parse error(s) and 0 validation error(s)\n   ╭─[1:16]\n 1 │ invalid-message\n   ·                ┬\n   ·                ╰── Expected a token starting with "="\n 2 │ \n 3 │ valid-message = I\'m valid.\n   ╰────\n'
    assert message == expected


def test_parser_error_str():
    assert str(fluent.ParserError) == "<class 'rustfluent.ParserError'>"


def test_parser_error_has_structured_error_details():
    """Test that ParserError exposes structured error information."""
    filename = data_dir / "errors.ftl"

    with pytest.raises(fluent.ParserError) as exc_info:
        fluent.Bundle("fr", [filename], strict=True)

    error = exc_info.value

    # Verify the parse_errors attribute exists
    assert hasattr(error, "parse_errors"), "ParserError should have 'parse_errors' attribute"
    assert len(error.parse_errors) == 1, "Should have exactly one error"

    # Verify the structure of the error detail
    error_detail = error.parse_errors[0]
    assert type(error_detail).__name__ == "ParseErrorDetail"

    # Verify all expected attributes exist and have correct values
    assert hasattr(error_detail, "message")
    assert hasattr(error_detail, "line")
    assert hasattr(error_detail, "column")
    assert hasattr(error_detail, "byte_start")
    assert hasattr(error_detail, "byte_end")
    assert hasattr(error_detail, "filename")

    # Verify the error is at the expected location
    assert error_detail.line == 1
    assert error_detail.column == 16
    assert error_detail.byte_start == 15
    assert error_detail.byte_end == 16

    # Verify the message contains expected content
    assert 'Expected a token starting with "="' in error_detail.message

    # Verify filename is included
    assert str(filename) in error_detail.filename

    # Verify string representation works
    error_str = str(error_detail)
    assert "1:16" in error_str  # Line:column should be in string representation
    assert "Expected a token" in error_str


# Attribute access tests


def test_basic_attribute_access():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    assert bundle.get_translation("welcome-message.title") == "Welcome to our site"


def test_regular_message_still_works_with_attributes():
    """Test that accessing the main message value still works when it has attributes."""
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    assert bundle.get_translation("welcome-message") == "Welcome!"


def test_multiple_attributes_on_same_message():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    assert bundle.get_translation("login-input.placeholder") == "email@example.com"
    assert bundle.get_translation("login-input.aria-label") == "Login input value"
    assert bundle.get_translation("login-input.title") == "Type your login email"


def test_attribute_with_variables():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    result = bundle.get_translation("greeting.formal", variables={"name": "Alice"})
    assert result == f"Hello, {BIDI_OPEN}Alice{BIDI_CLOSE}"


def test_attribute_with_variables_use_isolating_off():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    result = bundle.get_translation(
        "greeting.informal",
        variables={"name": "Bob"},
        use_isolating=False,
    )
    assert result == "Hi Bob!"


def test_attribute_on_message_without_main_value():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    assert bundle.get_translation("form-button.submit") == "Submit Form"
    assert bundle.get_translation("form-button.cancel") == "Cancel"
    assert bundle.get_translation("form-button.reset") == "Reset Form"


def test_message_without_value_raises_error():
    """Test that accessing a message without a value (only attributes) raises an error."""
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    with pytest.raises(ValueError, match="form-button - Message has no value"):
        bundle.get_translation("form-button")


def test_missing_message_with_attribute_syntax_raises_error():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    with pytest.raises(ValueError, match="nonexistent not found"):
        bundle.get_translation("nonexistent.title")


def test_missing_attribute_raises_error():
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    with pytest.raises(
        ValueError,
        match="welcome-message.nonexistent - Attribute 'nonexistent' not found on message 'welcome-message'",
    ):
        bundle.get_translation("welcome-message.nonexistent")


@pytest.mark.parametrize(
    "identifier,expected",
    (
        ("welcome-message", "Welcome!"),
        ("welcome-message.title", "Welcome to our site"),
        ("welcome-message.aria-label", "Welcome greeting"),
        ("login-input", "Email"),
        ("login-input.placeholder", "email@example.com"),
    ),
)
def test_attribute_and_message_access_parameterized(identifier, expected):
    bundle = fluent.Bundle("en", [data_dir / "attributes.ftl"])
    assert bundle.get_translation(identifier) == expected


# ==============================================================================
# Term Tests
# ==============================================================================
# Tests for Fluent terms - reusable vocabulary items that start with "-"
# and can only be referenced within messages (not retrieved directly)


def test_basic_term_reference():
    """Test that messages can reference terms using { -term-name } syntax."""
    bundle = fluent.Bundle("en", [data_dir / "terms.ftl"])
    result = bundle.get_translation("welcome")
    assert result == "Welcome to Acme Corporation!"


def test_term_attribute_as_selector():
    """Test that term attributes can be used as selectors in select expressions.

    Note: Per Fluent spec, term attributes are private and can only be used
    as selectors, not as direct placeables like { -term.attribute }.
    """
    bundle = fluent.Bundle("en", [data_dir / "terms.ftl"])
    result = bundle.get_translation("product-category")
    assert result == "This is a gadget"


def test_direct_term_access_fails():
    """Test that terms cannot be retrieved directly per Fluent spec."""
    bundle = fluent.Bundle("en", [data_dir / "terms.ftl"])
    with pytest.raises(ValueError, match="-brand-name not found"):
        bundle.get_translation("-brand-name")


def test_unknown_term_validation_error():
    """Test that referencing a non-existent term generates a validation error."""
    bundle = fluent.Bundle("en", [data_dir / "broken_terms.ftl"])

    # Should have validation errors
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) == 2  # One for unknown term, one for unknown attribute

    # Find the unknown term error
    unknown_term_errors = [e for e in validation_errors if e.error_type == "UnknownTerm"]
    assert len(unknown_term_errors) == 1

    error = unknown_term_errors[0]
    assert error.error_type == "UnknownTerm"
    assert "-nonexistent-term" in error.message or "nonexistent-term" in error.message
    assert error.message_id == "broken-reference"


def test_unknown_term_attribute_validation_error():
    """Test that referencing a non-existent term attribute generates a validation error."""
    bundle = fluent.Bundle("en", [data_dir / "broken_terms.ftl"])

    # Should have validation errors
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) == 2  # One for unknown term, one for unknown attribute

    # Find the unknown attribute error
    unknown_attr_errors = [e for e in validation_errors if e.error_type == "UnknownAttribute"]
    assert len(unknown_attr_errors) == 1

    error = unknown_attr_errors[0]
    assert error.error_type == "UnknownAttribute"
    assert "nonexistent" in error.message
    assert error.message_id == "broken-attribute"


def test_terms_across_multiple_files():
    """Test that terms defined in one file can be referenced in messages from another file."""
    # Load terms file first, then messages file
    bundle = fluent.Bundle(
        "en", [data_dir / "terms_definitions.ftl", data_dir / "terms_messages.ftl"]
    )

    # Should have no validation errors
    assert len(bundle.get_validation_errors()) == 0

    # Verify messages correctly reference terms from the other file
    assert bundle.get_translation("about-app") == "About SuperApp"
    assert bundle.get_translation("app-title") == "SuperApp - The Best App"
    assert bundle.get_translation("app-description") == "This is a Web App"
    assert bundle.get_translation("company-info") == "Brought to you by Acme Corporation"


def test_strict_mode_rejects_unknown_terms():
    """Test that strict mode raises an error when unknown terms are referenced.

    Note: When there are only validation errors (no parse errors), a ValueError
    is raised. When there are parse errors, a ParserError is raised.
    """
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("en", [data_dir / "broken_terms.ftl"], strict=True)

    error = exc_info.value

    # Should have validation errors
    assert hasattr(error, "validation_errors")
    assert len(error.validation_errors) == 2

    # Check that the errors mention the unknown term and attribute
    error_messages = [e.message for e in error.validation_errors]
    assert any("nonexistent-term" in msg for msg in error_messages)
    assert any("nonexistent" in msg and "attribute" in msg.lower() for msg in error_messages)


def test_term_positional_arguments_generate_validation_error():
    """Test that positional arguments to terms generate a validation error.

    Per Fluent spec, positional arguments to terms are syntactically valid but
    semantically ignored at runtime. We warn about them to prevent confusion.
    """
    bundle = fluent.Bundle("en", [data_dir / "term_positional_args.ftl"])

    # Should have validation errors for the two messages with positional args
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) == 2

    # Both errors should be about ignored positional arguments
    for error in validation_errors:
        assert error.error_type == "IgnoredPositionalArgument"
        assert "positional" in error.message.lower() or "ignored" in error.message.lower()
        assert "-brand-name" in error.message
        assert error.message_id in ["bad-reference", "bad-reference-mixed"]

    # The good reference should still work correctly
    assert bundle.get_translation("good-reference") == "About Firefoxie."


def test_strict_mode_rejects_term_positional_arguments():
    """Test that strict mode raises an error when terms are given positional arguments."""
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("en", [data_dir / "term_positional_args.ftl"], strict=True)

    error = exc_info.value

    # Should have validation errors
    assert hasattr(error, "validation_errors")
    assert len(error.validation_errors) == 2

    # Check that the errors mention positional arguments
    error_messages = [e.message for e in error.validation_errors]
    assert all("positional" in msg.lower() or "ignored" in msg.lower() for msg in error_messages)


def test_term_positional_arguments_validation_has_correct_context():
    """Test that positional argument validation errors have correct context."""
    bundle = fluent.Bundle("en", [data_dir / "term_positional_args.ftl"])

    validation_errors = bundle.get_validation_errors()

    # Find the error for bad-reference
    bad_ref_error = next(e for e in validation_errors if e.message_id == "bad-reference")

    assert bad_ref_error.error_type == "IgnoredPositionalArgument"
    assert bad_ref_error.message_id == "bad-reference"
    assert bad_ref_error.reference == "-brand-name"
    assert "named arguments" in bad_ref_error.message


# ==============================================================================
# Unknown Message Reference Tests
# ==============================================================================


def test_unknown_message_reference_validation_error():
    """Test that referencing a non-existent message generates a validation error."""
    bundle = fluent.Bundle("en", [data_dir / "broken_refs.ftl"])

    # Should have validation errors for the unknown message reference
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) >= 1

    # Find the unknown message error
    unknown_msg_errors = [e for e in validation_errors if e.error_type == "UnknownMessage"]
    assert len(unknown_msg_errors) == 1

    error = unknown_msg_errors[0]
    assert error.error_type == "UnknownMessage"
    assert "unknown-message" in error.message
    assert error.message_id == "msg-with-unknown-ref"
    assert error.reference == "unknown-message"


def test_unknown_message_reference_strict_mode():
    """Test that strict mode raises an error when unknown messages are referenced."""
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("en", [data_dir / "broken_refs.ftl"], strict=True)

    error = exc_info.value

    # Should have validation errors
    assert hasattr(error, "validation_errors")
    assert len(error.validation_errors) >= 1

    # Check that the error mentions the unknown message
    error_messages = [e.message for e in error.validation_errors]
    assert any("unknown-message" in msg for msg in error_messages)


def test_unknown_message_reference_non_strict_mode():
    """Test that non-strict mode allows unknown references but tracks them."""
    # Should not raise in non-strict mode
    bundle = fluent.Bundle("en", [data_dir / "broken_refs.ftl"], strict=False)

    # But should still track the validation errors
    validation_errors = bundle.get_validation_errors()
    unknown_msg_errors = [e for e in validation_errors if e.error_type == "UnknownMessage"]
    assert len(unknown_msg_errors) >= 1


# ==============================================================================
# Cyclic Reference Tests
# ==============================================================================


def test_cyclic_reference_validation_error():
    """Test that cyclic message references are detected as validation errors."""
    bundle = fluent.Bundle("en", [data_dir / "cycle.ftl"])

    # Should have validation errors for cyclic references
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) >= 1

    # Find cyclic reference errors
    cycle_errors = [e for e in validation_errors if e.error_type == "CyclicReference"]
    assert len(cycle_errors) >= 1

    # At least one error should mention the cycle
    error = cycle_errors[0]
    assert error.error_type == "CyclicReference"
    assert "cycle" in error.message.lower() or "cyclic" in error.message.lower()
    # Should mention both messages in the cycle
    assert "msg-a" in error.message and "msg-b" in error.message


def test_cyclic_reference_strict_mode():
    """Test that strict mode raises an error when cycles are detected."""
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("en", [data_dir / "cycle.ftl"], strict=True)

    error = exc_info.value

    # Should have validation errors
    assert hasattr(error, "validation_errors")
    assert len(error.validation_errors) >= 1

    # Check that the error mentions cycles
    error_messages = [e.message for e in error.validation_errors]
    assert any("cycle" in msg.lower() or "cyclic" in msg.lower() for msg in error_messages)


def test_cyclic_reference_non_strict_mode():
    """Test that non-strict mode allows cycles but tracks them."""
    # Should not raise in non-strict mode
    bundle = fluent.Bundle("en", [data_dir / "cycle.ftl"], strict=False)

    # But should still track the validation errors
    validation_errors = bundle.get_validation_errors()
    cycle_errors = [e for e in validation_errors if e.error_type == "CyclicReference"]
    assert len(cycle_errors) >= 1


# ==============================================================================
# Duplicate Message ID Tests
# ==============================================================================


def test_duplicate_message_id_validation_error():
    """Test that duplicate message IDs generate validation errors."""
    bundle = fluent.Bundle("en", [data_dir / "duplicates.ftl"])

    # Should have validation errors for duplicate message IDs
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) >= 1

    # Find duplicate message errors
    duplicate_errors = [e for e in validation_errors if e.error_type == "DuplicateMessageId"]
    assert len(duplicate_errors) >= 1

    # At least one error should mention the duplicate
    error = duplicate_errors[0]
    assert error.error_type == "DuplicateMessageId"
    assert "duplicate" in error.message.lower()
    assert "hello" in error.message  # The duplicated message ID


def test_duplicate_message_id_uses_last_definition():
    """Test that when messages are duplicated, the last definition wins."""
    bundle = fluent.Bundle("en", [data_dir / "duplicates.ftl"])

    # Should have validation errors but still work
    validation_errors = bundle.get_validation_errors()
    duplicate_errors = [e for e in validation_errors if e.error_type == "DuplicateMessageId"]
    assert len(duplicate_errors) >= 1

    # The last definition should be used (per Fluent spec with add_resource_overriding)
    result = bundle.get_translation("hello")
    assert "duplicate" in result.lower() or "second" in result.lower()


def test_duplicate_message_id_strict_mode():
    """Test that strict mode raises an error when duplicates are detected."""
    with pytest.raises(ValueError) as exc_info:
        fluent.Bundle("en", [data_dir / "duplicates.ftl"], strict=True)

    error = exc_info.value

    # Should have validation errors
    assert hasattr(error, "validation_errors")
    assert len(error.validation_errors) >= 1

    # Check that the error mentions duplicates
    error_messages = [e.message for e in error.validation_errors]
    assert any("duplicate" in msg.lower() for msg in error_messages)


def test_duplicate_message_id_non_strict_mode():
    """Test that non-strict mode allows duplicates but tracks them."""
    # Should not raise in non-strict mode
    bundle = fluent.Bundle("en", [data_dir / "duplicates.ftl"], strict=False)

    # But should still track the validation errors
    validation_errors = bundle.get_validation_errors()
    duplicate_errors = [e for e in validation_errors if e.error_type == "DuplicateMessageId"]
    assert len(duplicate_errors) >= 1


# ==============================================================================
# Error Collection Methods Tests
# ==============================================================================


def test_get_parse_errors():
    """Test that get_parse_errors() returns syntax errors from FTL parsing."""
    bundle = fluent.Bundle("fr", [data_dir / "errors.ftl"], strict=False)

    # Should have parse errors
    parse_errors = bundle.get_parse_errors()
    assert len(parse_errors) == 1

    # Verify the error structure
    error = parse_errors[0]
    assert hasattr(error, "message")
    assert hasattr(error, "line")
    assert hasattr(error, "column")
    assert hasattr(error, "byte_start")
    assert hasattr(error, "byte_end")
    assert hasattr(error, "filename")

    # Verify error is about the missing =
    assert error.line == 1
    assert error.column == 16
    assert 'Expected a token starting with "="' in error.message


def test_get_all_compile_errors_combined():
    """Test that get_all_compile_errors() returns both parse and validation errors with tags."""
    bundle = fluent.Bundle("fr", [data_dir / "mixed_errors.ftl"], strict=False)

    # Get all errors
    all_errors = bundle.get_all_compile_errors()

    # Should have both parse and validation errors
    assert len(all_errors) >= 2

    # Separate by category
    parse_errors = [e for category, e in all_errors if category == "parse"]
    validation_errors = [e for category, e in all_errors if category == "validation"]

    # Should have at least one of each
    assert len(parse_errors) >= 1
    assert len(validation_errors) >= 1

    # Parse errors should be ParseErrorDetail instances
    assert type(parse_errors[0]).__name__ == "ParseErrorDetail"

    # Validation errors should be ValidationError instances
    assert type(validation_errors[0]).__name__ == "ValidationError"


def test_get_all_compile_errors_only_parse():
    """Test get_all_compile_errors() with only parse errors."""
    bundle = fluent.Bundle("fr", [data_dir / "errors.ftl"], strict=False)

    all_errors = bundle.get_all_compile_errors()
    parse_errors = [e for category, e in all_errors if category == "parse"]
    validation_errors = [e for category, e in all_errors if category == "validation"]

    assert len(parse_errors) >= 1
    assert len(validation_errors) == 0


def test_get_all_compile_errors_only_validation():
    """Test get_all_compile_errors() with only validation errors."""
    bundle = fluent.Bundle("en", [data_dir / "broken_refs.ftl"], strict=False)

    all_errors = bundle.get_all_compile_errors()
    parse_errors = [e for category, e in all_errors if category == "parse"]
    validation_errors = [e for category, e in all_errors if category == "validation"]

    assert len(parse_errors) == 0
    assert len(validation_errors) >= 1


# ==============================================================================
# get_required_variables() Tests
# ==============================================================================


def test_get_required_variables_single_variable():
    """Test extracting a single variable from a message."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    variables = bundle.get_required_variables("greeting")

    assert len(variables) == 1
    assert "name" in variables


def test_get_required_variables_multiple_variables():
    """Test extracting multiple variables from a message."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    variables = bundle.get_required_variables("user-info")

    assert len(variables) == 2
    assert "username" in variables
    assert "count" in variables
    # Should be sorted
    assert variables == sorted(variables)


def test_get_required_variables_with_selector():
    """Test extracting variables from messages with selectors."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    variables = bundle.get_required_variables("item-status")

    # Should include variables from selector and all variants
    assert "count" in variables
    assert "user" in variables
    assert len(variables) == 2


def test_get_required_variables_from_attribute():
    """Test extracting variables from message attributes."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])

    # Test subject attribute
    subject_vars = bundle.get_required_variables("email-template.subject")
    assert "recipient" in subject_vars

    # Test body attribute
    body_vars = bundle.get_required_variables("email-template.body")
    assert "messageCount" in body_vars


def test_get_required_variables_message_not_found():
    """Test that get_required_variables raises error for non-existent message."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])

    with pytest.raises(ValueError, match="nonexistent not found"):
        bundle.get_required_variables("nonexistent")


# ==============================================================================
# validate_references Parameter Tests
# ==============================================================================


def test_validate_references_disabled():
    """Test that validate_references=False skips reference validation."""
    # This file has unknown message references, but validation should be skipped
    bundle = fluent.Bundle(
        "en", [data_dir / "broken_refs.ftl"], strict=False, validate_references=False
    )

    # Should have NO validation errors because validation was disabled
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) == 0


def test_validate_references_enabled_by_default():
    """Test that validate_references defaults to True."""
    # Same file, but validation should run by default
    bundle = fluent.Bundle("en", [data_dir / "broken_refs.ftl"], strict=False)

    # Should have validation errors
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) >= 1


def test_validate_references_false_with_cycles():
    """Test that cycles are also skipped when validate_references=False."""
    bundle = fluent.Bundle("en", [data_dir / "cycle.ftl"], strict=False, validate_references=False)

    # Should have NO validation errors
    validation_errors = bundle.get_validation_errors()
    assert len(validation_errors) == 0


def test_validate_references_false_with_duplicates():
    """Test that duplicates are still detected even with validate_references=False."""
    bundle = fluent.Bundle(
        "en", [data_dir / "duplicates.ftl"], strict=False, validate_references=False
    )

    # Duplicates should still be detected (they're checked before validate_references)
    validation_errors = bundle.get_validation_errors()
    duplicate_errors = [e for e in validation_errors if e.error_type == "DuplicateMessageId"]
    assert len(duplicate_errors) >= 1


# ==============================================================================
# get_translation() errors Parameter Tests
# ==============================================================================


def test_get_translation_errors_missing_variable():
    """Test that missing variables are reported via errors parameter."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    errors: list[fluent.FormatError] = []

    # Call get_translation without providing required variable
    result = bundle.get_translation("greeting", errors=errors)

    # Result should still work (using fallback)
    assert "name" in result

    # Should have a MissingVariable error
    assert len(errors) >= 1
    error = errors[0]
    assert error.error_type == "MissingVariable"
    assert error.variable_name == "name"
    assert error.message_id == "greeting"
    assert "Unknown external: name" in error.message


def test_get_translation_errors_invalid_variable_type():
    """Test that invalid variable types are reported via errors parameter."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    errors: list[fluent.FormatError] = []

    # Pass a list instead of a string
    result = bundle.get_translation(
        "greeting",
        variables={"name": ["not", "a", "string"]},  # type: ignore[dict-item]
        errors=errors,
    )

    # Result should use fallback (the variable key itself)
    assert "name" in result

    # Should have an InvalidVariableType error
    assert len(errors) >= 1
    error = errors[0]
    assert error.error_type == "InvalidVariableType"
    assert error.variable_name == "name"
    assert error.message_id == "greeting"
    assert error.expected_type == "str|int|date"
    assert error.actual_type is not None


def test_get_translation_errors_multiple_missing_variables():
    """Test that multiple missing variables are all reported."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    errors: list[fluent.FormatError] = []

    # user-info needs both $username and $count
    result = bundle.get_translation("user-info", errors=errors)
    # Result should still contain placeholders
    assert "username" in result or "count" in result

    # Should have 2 MissingVariable errors
    missing_errors = [e for e in errors if e.error_type == "MissingVariable"]
    assert len(missing_errors) == 2

    variable_names = {e.variable_name for e in missing_errors}
    assert "username" in variable_names
    assert "count" in variable_names


def test_get_translation_errors_partial_variables():
    """Test errors when only some variables are provided."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    errors: list[fluent.FormatError] = []

    # Provide only username, not count
    result = bundle.get_translation("user-info", variables={"username": "Alice"}, errors=errors)

    # Should have formatted username correctly
    assert "Alice" in result

    # Should have 1 MissingVariable error for count
    missing_errors = [e for e in errors if e.error_type == "MissingVariable"]
    assert len(missing_errors) == 1
    assert missing_errors[0].variable_name == "count"


def test_get_translation_errors_none_parameter():
    """Test that errors=None doesn't break (errors are just not collected)."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])

    # This should not raise even though variable is missing
    result = bundle.get_translation("greeting", errors=None)
    assert "name" in result


def test_get_translation_errors_with_attributes():
    """Test error collection with message attributes."""
    bundle = fluent.Bundle("en", [data_dir / "variables.ftl"])
    errors: list[fluent.FormatError] = []

    # email-template.subject needs $recipient
    result = bundle.get_translation("email-template.subject", errors=errors)
    # Result should contain the placeholder
    assert "recipient" in result

    # Should have MissingVariable error
    missing_errors = [e for e in errors if e.error_type == "MissingVariable"]
    assert len(missing_errors) == 1
    assert missing_errors[0].variable_name == "recipient"
    assert missing_errors[0].message_id == "email-template.subject"
