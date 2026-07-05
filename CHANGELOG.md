# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Core workspace scanner and heuristic detection algorithms.
- Cross-platform filesystem watcher daemon supporting Windows, macOS, and Linux.
- Sweeper engine to identify and clean `node_modules` and `target` directories safely.
- Archivist module to provide zip-based backups before executing sweeps.
- Inference module stub utilizing HuggingFace local models.
- Layered config engine merging global, workspace, and local configuration logic.

### Changed
- Refactored core types and domain entities to ensure single source of truth across crates.

### Fixed
- Stabilized Windows symlink resolution and UNC path constraints.

## [0.1.0] - 2026-07-01
### Added
- Initial project layout and workspace setup.
