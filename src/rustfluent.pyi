from datetime import date

Variable = str | int | date

class Bundle:
    def __init__(self, language: str, ftl_filenames: list[str], strict: bool = False) -> None: ...
    def get_translation(
        self,
        identifier: str,
        variables: dict[str, Variable] | None = None,
        use_isolating: bool = True,
    ) -> str: ...
