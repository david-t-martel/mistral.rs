# WinUtils Documentation

## 📚 Documentation Structure

This documentation provides comprehensive technical information about the WinUtils project - a Windows-optimized GNU coreutils implementation with 77 utilities achieving 70-75% performance improvement over native Windows utilities.

### Core Documentation

| Document                                 | Description                                               |
| ---------------------------------------- | --------------------------------------------------------- |
| [**ARCHITECTURE.md**](ARCHITECTURE.md)   | Complete technical architecture deep-dive                 |
| [**API_REFERENCE.md**](API_REFERENCE.md) | Comprehensive API documentation for all public interfaces |
| [**PERFORMANCE.md**](PERFORMANCE.md)     | Performance benchmarks, metrics, and optimization guides  |
| [**CONTRIBUTING.md**](CONTRIBUTING.md)   | Development guidelines and contribution process           |

### User Guides

| Guide                                               | Description                                 |
| --------------------------------------------------- | ------------------------------------------- |
| [**GETTING_STARTED.md**](guides/GETTING_STARTED.md) | Quick start guide for new users             |
| [**INSTALLATION.md**](guides/INSTALLATION.md)       | Platform-specific installation instructions |
| [**MIGRATION.md**](guides/MIGRATION.md)             | Migration guide from standard utilities     |
| [**TROUBLESHOOTING.md**](guides/TROUBLESHOOTING.md) | Common issues and solutions                 |
| [**FAQ.md**](guides/FAQ.md)                         | Frequently asked questions                  |

### Developer Documentation

| Document                                         | Description                         |
| ------------------------------------------------ | ----------------------------------- |
| [**INTEGRATION.md**](developer/INTEGRATION.md)   | Guide for adding new utilities      |
| [**OPTIMIZATION.md**](developer/OPTIMIZATION.md) | Performance optimization techniques |
| [**TESTING.md**](developer/TESTING.md)           | Testing strategies and frameworks   |
| [**CI_CD.md**](developer/CI_CD.md)               | CI/CD pipeline documentation        |
| [**BUILD_SYSTEM.md**](developer/BUILD_SYSTEM.md) | Detailed build system documentation |

### Component Documentation

| Component                                         | Description                              |
| ------------------------------------------------- | ---------------------------------------- |
| [**winpath/**](components/winpath.md)             | Universal path normalization library     |
| [**winutils-core/**](components/winutils-core.md) | Enhanced features framework              |
| [**derive-utils/**](components/derive-utils.md)   | Windows-specific utilities documentation |
| [**coreutils/**](components/coreutils.md)         | GNU coreutils implementations            |

### Reference Documentation

| Reference                                          | Description                              |
| -------------------------------------------------- | ---------------------------------------- |
| [**CLI_REFERENCE.md**](reference/CLI_REFERENCE.md) | Complete CLI reference for all utilities |
| [**COMPATIBILITY.md**](reference/COMPATIBILITY.md) | GNU compatibility matrix                 |
| [**BENCHMARKS.md**](reference/BENCHMARKS.md)       | Detailed performance benchmarks          |
| [**CHANGELOG.md**](reference/CHANGELOG.md)         | Version history and changes              |

## 🚀 Quick Links

- **Getting Started**: [GETTING_STARTED.md](guides/GETTING_STARTED.md)
- **API Documentation**: [API_REFERENCE.md](API_REFERENCE.md)
- **Architecture Overview**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Performance Guide**: [PERFORMANCE.md](PERFORMANCE.md)

## 📊 Project Statistics

- **Total Utilities**: 77 (74 GNU coreutils + 3 Windows derive utilities)
- **Performance Improvement**: 70-75% faster than native Windows utilities
- **Path Normalization**: \<1ms with LRU caching
- **Build Time**: 2-3 minutes full, \<30 seconds incremental
- **Binary Size**: Average 1.16 MB per utility
- **Test Coverage**: >85% code coverage
- **Platform Support**: Windows 10/11, WSL, Git Bash, Cygwin

## 🏗️ Technology Stack

- **Language**: Rust 2021 Edition
- **Build System**: GNU Make (Makefile orchestration)
- **Path Handling**: Universal winpath library
- **Windows APIs**: windows-sys 0.52, windows 0.60
- **Testing**: Rust native tests + PowerShell validation
- **Optimization**: LTO, native CPU targeting, SIMD where applicable

## 📝 Documentation Standards

All documentation follows these principles:

1. **Clarity**: Clear, concise technical writing
1. **Completeness**: Comprehensive coverage of all aspects
1. **Examples**: Code examples for all major features
1. **Diagrams**: ASCII art diagrams for architecture
1. **Maintenance**: Regular updates with each release
1. **Accessibility**: Progressive disclosure of complexity

## 🔗 External Resources

- [GNU Coreutils Documentation](https://www.gnu.org/software/coreutils/manual/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [Windows API Documentation](https://docs.microsoft.com/en-us/windows/win32/)
- [Git Bash Documentation](https://git-scm.com/doc)

## 📧 Contact

- **Author**: David Martel
- **Email**: david.martel@auricleinc.com
- **Repository**: [GitHub - WinUtils Windows Fork](https://github.com/david-t-martel/uutils-windows)

______________________________________________________________________

*Documentation Version: 1.0.0*
*Last Updated: January 2025*
*Maintained with Claude Code Framework*
