"""
This `noxfile.py` is configured to run the test suite with multiple versions of Python.

If you want to also run the test suite against multiple version of some dependency (e.g. Django),
use the `noxfile.with-packages.py` file instead.
"""

import os

import nox


# Use uv to manage venvs.
nox.options.default_venv_backend = "uv"


@nox.session()
@nox.parametrize(
    "python",
    [
        nox.param("3.11", id="python=3.11"),
        nox.param("3.12", id="python=3.12"),
    ],
)
def tests(session: nox.Session) -> None:
    """
    Run the test suite.
    """
    # Install the development dependencies.
    session.install("-r", "requirements/development.txt", ".")

    # When run in CircleCI, create JUnit XML test results.
    commands = ["pytest"]
    if "CIRCLECI" in os.environ:
        commands.append(f"--junitxml=test-results/junit.{session.name}.xml")

    session.run(*commands, *session.posargs)
