# Changelog and Versioning

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Changed error handling during `Bundle` instantiation. Now message errors will be ignored by default, overrideable
  by a new `strict` parameter. In this mode, a `ParserError` will be raised instead of a `ValueError` as before.
- Renamed the `namespace` argument to `language`.
- Fluent message variables are now no longer passed to `get_translation` using `**kwargs`; instead a `variables`
  parameter is used.

## [0.1.0a1] - 2024-09-13

- Added initial implementation.
