# Changelog

Changelog following the template from https://keepachangelog.com.
All versions before 1.X.X can contain major changes.

## [0.4.2] - 2024-03-24

- Updated dependencies
- Added a maximum retry interval of 5 seconds instead of a minute.

## [0.4.1] - 2023-06-23

### Fixed

- Repository location and homepage now point to GitLab.

## [0.4.0] - 2023-06-23

### Changed

- Complete overhaul. Added tracing, backoff and made sure all methods are infallible (won't panic).
