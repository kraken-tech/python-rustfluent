from datetime import date
from pathlib import Path

Variable = str | int | date

class ParseErrorDetail:
    """Parse time: syntax errors in .ftl files (invalid FTL syntax, malformed messages)"""

    message: str
    line: int
    column: int
    byte_start: int
    byte_end: int
    filename: str | None

class ValidationError:
    """Load time: semantic errors during Bundle() init (unknown refs, cycles, duplicate IDs)"""

    error_type: (
        str  # DuplicateMessageId, UnknownMessage, UnknownTerm, UnknownAttribute, CyclicReference
    )
    message: str
    message_id: str | None
    reference: str | None

class FormatError:
    """Runtime: errors during get_translation() (missing variables, invalid variable types)"""

    error_type: str  # MissingVariable, InvalidVariableType
    message: str
    message_id: str | None
    variable_name: str | None
    expected_type: str | None
    actual_type: str | None

class Bundle:
    def __init__(
        self,
        language: str,
        ftl_filenames: list[str | Path],
        strict: bool = False,
        validate_references: bool = True,
    ) -> None: ...
    def get_translation(
        self,
        identifier: str,
        variables: dict[str, Variable] | None = None,
        use_isolating: bool = True,
        errors: list[FormatError] | None = None,
    ) -> str: ...
    def get_parse_errors(self) -> list[ParseErrorDetail]: ...
    def get_validation_errors(self) -> list[ValidationError]: ...
    def get_all_compile_errors(
        self,
    ) -> list[tuple[str, ParseErrorDetail | ValidationError]]: ...
    def get_compile_errors(self) -> list[ValidationError]: ...
    def get_required_variables(self, identifier: str) -> list[str]: ...
