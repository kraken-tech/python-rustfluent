from datetime import date
from pathlib import Path

Variable = str | int | date

class Bundle:
    def __init__(self, language: str, ftl_filename: str | Path, strict: bool = False) -> None: ...
    def get_translation(
        self,
        identifier: str,
        variables: dict[str, Variable] | None = None,
        use_isolating: bool = True,
    ) -> str: ...
