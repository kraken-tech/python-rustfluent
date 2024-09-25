# Contributing

During development you will also need the following system packages installed:

- [uv](https://astral.sh/blog/uv) - for Python packaging.
- Cargo - for Rust development. (These can be installed using [rustup](https://www.rust-lang.org/tools/install)).

## Local development

When making changes please remember to update the `CHANGELOG.md`, which follows the guidelines at
[keepachangelog]. Add your changes to the `[Unreleased]` section when you create your PR.

[keepachangelog]: https://keepachangelog.com/

### Installation

Ensure one of the above Pythons is installed and used by the `python` executable:

```sh
python --version
Python 3.11.9   # or any of the supported versions
```

Ensure `uv` is installed as a system package. This can be done with `pipx` or Homebrew.

Then create and activate a virtual environment. If you don't have any other way of managing virtual
environments this can be done by running:

```sh
uv venv
source .venv/bin/activate
```

You could also use [virtualenvwrapper], [direnv] or any similar tool to help manage your virtual
environments.

Once you are in an active virtual environment run

```sh
make dev
```

This will set up your local development environment, installing all development dependencies.

[virtualenvwrapper]: https://virtualenvwrapper.readthedocs.io/
[direnv]: https://direnv.net

### Testing (single Python version)

Run all Rust and Python tests (within your specific virtual environment) using this command:

```sh
make test
```

This will recompile the Rust code (if necessary) before running the tests.

#### Using pytest directly

You can also run tests using pytest directly (e.g. via your IDE), but you will need to recompile the Rust code after
any changes, otherwise they won't get picked up. You can recompile by running:

```sh
maturin develop
```

### Testing (all supported Python versions)

To test against multiple Python (and package) versions, we need to:

- Have [`nox`][nox] installed outside of the virtualenv. This is best done using `pipx`:

  ```sh
  pipx install nox
  ```

- Ensure that all supported Python versions are installed and available on your system (as e.g.
  `python3.10`, `python3.11` etc). This can be done with `pyenv`.

Then run `nox` with:

```sh
nox
```

Nox will create a separate virtual environment for each combination of Python and package versions
defined in `noxfile.py`.

To list the available sessions, run:

```sh
nox --list-sessions
```

To run the test suite in a specific Nox session, use:

```sh
nox -s $SESSION_NAME
```

[nox]: https://nox.thea.codes/en/stable/

### Static analysis

Run all static analysis tools with:

```sh
make lint
```

### Auto formatting

Reformat code to conform with our conventions using:

```sh
make format
```

### Dependencies

Package dependencies are declared in `pyproject.toml`.

- _package_ dependencies in the `dependencies` array in the `[project]` section.
- _development_ dependencies in the `dev` array in the `[project.optional-dependencies]` section.

For local development, the dependencies declared in `pyproject.toml` are pinned to specific
versions using the `requirements/development.txt` lock file.

#### Adding a new dependency

To install a new Python dependency add it to the appropriate section in `pyproject.toml` and then
run:

```sh
make dev
```

This will:

1. Build a new version of the `requirements/development.txt` lock file containing the newly added
   package.
2. Sync your installed packages with those pinned in `requirements/development.txt`.

This will not change the pinned versions of any packages already in any requirements file unless
needed by the new packages, even if there are updated versions of those packages available.

Remember to commit your changed `requirements/development.txt` files alongside the changed
`pyproject.toml`.

#### Removing a dependency

Removing Python dependencies works exactly the same way: edit `pyproject.toml` and then run
`make dev`.

#### Updating all Python packages

To update the pinned versions of all packages simply run:

```sh
make update
```

This will update the pinned versions of every package in the `requirements/development.txt` lock
file to the latest version which is compatible with the constraints in `pyproject.toml`.

You can then run:

```sh
make dev
```

to sync your installed packages with the updated versions pinned in `requirements/development.txt`.

#### Updating individual Python packages

Upgrade a single development dependency with:

```sh
uv pip compile -P $PACKAGE==$VERSION pyproject.toml --extra=dev --output-file=requirements/development.txt
```

You can then run:

```sh
make dev
```

to sync your installed packages with the updated versions pinned in `requirements/development.txt`.

## How to release a new version to Pypi

1. Create a new branch off the `main` branch.
2. Double check that the `Unreleased` section of CHANGELOG.md is up to date. If it isn't, add a short summary
   of changes.
3. Run the appropriate command, depending on which kind of release it is:
   - either `make version_major`
   - or `make version_minor`
   - or `make version_patch`.
4. Review the commit that has just been made (note it will have updated the version based on [SemVer]).
5. Push the commit, get it reviewed and merge it to `main`.
6. Once it's merged, tag the merge commit and push tags:
   - `git checkout main && git pull`
   - `git tag YOUR_VERSION` (In the form `v0.0.0`.)
   - `git push --tags`
7. Keep an eye on the [release workflow] which should have started. Check that it completes, and the latest version has
   made its way onto [Pypi](https://pypi.org/project/rustfluent/).

[semver]: https://semver.org/
[release workflow]: https://github.com/kraken-tech/python-rustfluent/actions/workflows/release.yml
