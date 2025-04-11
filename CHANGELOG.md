# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),  
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.0.3] - 2025-04-11
- update dependencies
- small UI improvements
- Optimization of memory usage
- reduce the file size of the `search_result.json` file by removing pretty formatted JSON

### Added
- It is now possible to copy the `About-Window` information as JSON to the clipboard by pressing `Ctrl+C`
<br>


## [1.0.2] - 2025-03-23
- update dependencies

### Added
- pre compiled binaries for Linux, MacOs and Windows
  - provided builds for `x86_64` and `aarch_64(ARM64)`
<br>


## [1.0.1] - 2025-02-21
### Fixed
- Improved filtering logic for selecting files and directories by their initial letter, with proper case-insensitive matching.
- Correctly handles non-ASCII characters when filtering by initial letters
<br>


## [1.0.0] - 2025-02-20
### Added
- 🚀 Initial release of the `TraceView` application.
- 📂 Filesystem explorer: Navigate the local filesystem with ease.
- 📊 System overview: View real-time system resource usage (CPU, memory, disk).
- 🔎 File and directory search: Search for files and directories by name.
- 📝 Export functionality: Save search results as a JSON file.
- 🛠️ Configurable settings via a `config.toml` file.
- 🗂️ Metadata retrieval: View metadata for files and directories (size, permissions, last modified, and more).
- ❓ Comprehensive help page accessible within the application.

### Fixed
- N/A (Initial release)

### Changed
- N/A
