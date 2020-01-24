# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3] - 2020-01-24
### Security
- Security fixes

## [0.5.2] - 2019-09-03
### Security
- Security fixes

## [0.5.1] - 2019-07-09
### Security
- Security fixes

## [0.5.0] - 2019-04-18
### Added
- Parsing URLs before shortening them, therefore checking that they are valid
- Link loop check: an attempt to shorten a url with the same domain used by shorty to serve shortened urls will result in an error

## [0.4.0] - 2019-04-10
### Fixed
- Another `RedisFacade.get_bool` bug
### Added
- Conflicting ID check and multiple attempts to generate a unique one

## [0.3.0] - 2019-04-09
### Fixed
- `RedisFacade.get_bool` bug
### Added
- Documentation

## [0.2.1] - 2019-04-09
### Added
- Also publishing docker `latest` image

## [0.2.0] - 2019-04-09
- First implementation
