# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.7] - 2024-04-04
### Fixed
- do not unwrap during shutdown

## [0.1.6] - 2024-04-01
### Fixed
- always print error information

## [0.1.5] - 2024-04-01
### Fixed
- do not println debugging logs when compiled in release mode
- make worker type cloneable

## [0.1.4] - 2024-03-11
### Feature
- Change embed color based on trace level

## [0.1.3] - 2023-07-20
### Fixed
- Handle network failures in message delivery with retry and exponential backoff

## [0.1.2] - 2023-06-19
### Changed
- Removed titles from fields

## [0.1.1] - 2023-06-19
### Changed
- Chunk field value's above 1k chars, truncate messages above 2k chars


## [0.1.0] - 2023-06-18
### Added
- Initial release of the discord layer

