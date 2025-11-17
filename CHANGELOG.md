# Changelog

## v0.5.7

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.6...v0.5.7)

### 🔥 Performance

- Change dashmap hasher in `TaskManager` to `rustc_hash::FxBuildHasher` ([0fd63d5](https://github.com/kikiutils/rust/commit/0fd63d5))

### 🏡 Chore

- Update Cargo.toml ([50b1056](https://github.com/kikiutils/rust/commit/50b1056))
- Update deps ([4c04ee9](https://github.com/kikiutils/rust/commit/4c04ee9))

### ❤️ Contributors

- Kiki-kanri

## v0.5.6

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.5...v0.5.6)

### 💅 Refactors

- Inline some generic type constraints instead of defining them in `where` clauses ([4fc790d](https://github.com/kikiutils/rust/commit/4fc790d))
- Change `pub(in crate::task)` to `pub(super)` ([9b69e4e](https://github.com/kikiutils/rust/commit/9b69e4e))

### 🏡 Chore

- Upgrade dependencies ([1f25c8c](https://github.com/kikiutils/rust/commit/1f25c8c))
- Unify feature definitions by using `dep:` prefix for all dependencies ([6b85cdd](https://github.com/kikiutils/rust/commit/6b85cdd))

### ❤️ Contributors

- Kiki-kanri

## v0.5.5

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.4...v0.5.5)

### 💅 Refactors

- Modify `wait_for_shutdown_signal` to merge Unix-related blocks and avoid importing `select` on non-Unix platforms ([c728c73](https://github.com/kikiutils/rust/commit/c728c73))

### 🤖 CI

- Change install cargo-llvm-cov method ([66a3c60](https://github.com/kikiutils/rust/commit/66a3c60))

### ❤️ Contributors

- Kiki-kanri

## v0.5.4

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.3...v0.5.4)

### 🏡 Chore

- Remove makefile ([d7f0419](https://github.com/kikiutils/rust/commit/d7f0419))

### 🤖 CI

- Add test on release and upload codecov workflow ([d782468](https://github.com/kikiutils/rust/commit/d782468))

### ❤️ Contributors

- Kiki-kanri

## v0.5.3

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.2...v0.5.3)

### 🎨 Styles

- Update formatting rules related to `use` and reformat all code ([70e593a](https://github.com/kikiutils/rust/commit/70e593a))

### ❤️ Contributors

- Kiki-kanri

## v0.5.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.1...v0.5.2)

### 🏡 Chore

- Change package setting from `exclude` to `include` ([840ee89](https://github.com/kikiutils/rust/commit/840ee89))

### ❤️ Contributors

- Kiki-kanri

## v0.5.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.0...v0.5.1)

### 🚀 Enhancements

- Add methods to `TaskManager` and mark some methods is inline ([1a2355b](https://github.com/kikiutils/rust/commit/1a2355b))

### 💅 Refactors

- Chore: tidy up `src/extensions/anyhow.rs` ([a4c1af6](https://github.com/kikiutils/rust/commit/a4c1af6))
- Tidy up code ([109c0d0](https://github.com/kikiutils/rust/commit/109c0d0))

### 🏡 Chore

- Update release script ([b49700a](https://github.com/kikiutils/rust/commit/b49700a))
- Update vscode settings ([4496409](https://github.com/kikiutils/rust/commit/4496409))

### ❤️ Contributors

- Kiki-kanri

## v0.5.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.4.0...v0.5.0)

### 💅 Refactors

- ⚠️ Update `TaskManager` ([a52307f](https://github.com/kikiutils/rust/commit/a52307f))

### 🏡 Chore

- Remove unused cargo alias config ([618987f](https://github.com/kikiutils/rust/commit/618987f))
- Update release script ([5ab9103](https://github.com/kikiutils/rust/commit/5ab9103))

#### ⚠️ Breaking Changes

- ⚠️ Update `TaskManager` ([a52307f](https://github.com/kikiutils/rust/commit/a52307f))

### ❤️ Contributors

- kiki-kanri

## v0.4.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.3.4...v0.4.0)

### 🚀 Enhancements

- Add new methods to the `TaskManager` ([605a814](https://github.com/kikiutils/rust/commit/605a814))
- Add Default implementation for `TaskManager` ([682777d](https://github.com/kikiutils/rust/commit/682777d))
- ⚠️ Update TaskManager ([5638e58](https://github.com/kikiutils/rust/commit/5638e58))
- Add `abort` method to the `TaskManager` ([737f821](https://github.com/kikiutils/rust/commit/737f821))

### 💅 Refactors

- ⚠️ Restructure project files and configure features with all disabled by default ([2d59437](https://github.com/kikiutils/rust/commit/2d59437))
- ⚠️ Redesign task manager ([ebd8f28](https://github.com/kikiutils/rust/commit/ebd8f28))

### 🏡 Chore

- Disable automatic formatting for TOML files ([caf5375](https://github.com/kikiutils/rust/commit/caf5375))
- Add `.editorconfig` ([3d0d31c](https://github.com/kikiutils/rust/commit/3d0d31c))
- Update scripts ([379c6a5](https://github.com/kikiutils/rust/commit/379c6a5))
- Add cargo alias config ([c84e8ac](https://github.com/kikiutils/rust/commit/c84e8ac))
- Upgrade dependencies ([5695201](https://github.com/kikiutils/rust/commit/5695201))
- Update release script ([6f64388](https://github.com/kikiutils/rust/commit/6f64388))

### ✅ Tests

- Update task manager unit tests ([2ff9a28](https://github.com/kikiutils/rust/commit/2ff9a28))
- Update task manager unit tests ([b01dbaa](https://github.com/kikiutils/rust/commit/b01dbaa))
- Update task manager unit ([ff4eb1b](https://github.com/kikiutils/rust/commit/ff4eb1b))

#### ⚠️ Breaking Changes

- ⚠️ Update TaskManager ([5638e58](https://github.com/kikiutils/rust/commit/5638e58))
- ⚠️ Restructure project files and configure features with all disabled by default ([2d59437](https://github.com/kikiutils/rust/commit/2d59437))
- ⚠️ Redesign task manager ([ebd8f28](https://github.com/kikiutils/rust/commit/ebd8f28))

### ❤️ Contributors

- kiki-kanri

## v0.3.4

[compare changes](https://github.com/kikiutils/rust/compare/v0.3.3...v0.3.4)

### 🏡 Chore

- Format code ([53aade3](https://github.com/kikiutils/rust/commit/53aade3))

### ❤️ Contributors

- kiki-kanri

## v0.3.3

[compare changes](https://github.com/kikiutils/rust/compare/v0.3.2...v0.3.3)

### 🏡 Chore

- Update dependencies and tidy up features ([3b74070](https://github.com/kikiutils/rust/commit/3b74070))
- Update release script ([bae5a47](https://github.com/kikiutils/rust/commit/bae5a47))

### ❤️ Contributors

- kiki-kanri

## v0.3.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.3.1...v0.3.2)

## v0.3.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.3.0...v0.3.1)

### 💅 Refactors

- Change `tokio::select!` to use and `select!` ([685bfec](https://github.com/kikiutils/rust/commit/685bfec))

### 🏡 Chore

- Update `.gitignore` and `modify-files-permissions.sh` ([520a2ac](https://github.com/kikiutils/rust/commit/520a2ac))
- Format `release.sh` ([2e0a027](https://github.com/kikiutils/rust/commit/2e0a027))
- Upgrade dependencies ([bc20e2a](https://github.com/kikiutils/rust/commit/bc20e2a))
- Upgrade dependencies ([e98920a](https://github.com/kikiutils/rust/commit/e98920a))

### ❤️ Contributors

- kiki-kanri

## v0.3.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.2.2...v0.3.0)

### 🚀 Enhancements

- Add `OptionAnyhowExt` and `ResultAnyhowExt` trait and impl ([319cbd2](https://github.com/kikiutils/rust/commit/319cbd2))

### ❤️ Contributors

- kiki-kanri

## v0.2.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.2.1...v0.2.2)

### 🏡 Chore

- Update CHANGELOG repository URL ([0dd73f4](https://github.com/kikiutils/rust/commit/0dd73f4))
- Upgrade dependencies ([699ace5](https://github.com/kikiutils/rust/commit/699ace5))
- Set rustfmt `max_width` to 120 ([0bb5135](https://github.com/kikiutils/rust/commit/0bb5135))
- Update `modify-files-permissions.sh` script ([c198025](https://github.com/kikiutils/rust/commit/c198025))

### ❤️ Contributors

- kiki-kanri

## v0.2.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.2.0...v0.2.1)

### 🏡 Chore

- Update dependencies ([447e644](https://github.com/kikiutils/rust/commit/447e644))
- Update repository URL ([91c6f5e](https://github.com/kikiutils/rust/commit/91c6f5e))
- Set `hideAuthorEmail` arg in changelogen command ([f2a5b92](https://github.com/kikiutils/rust/commit/f2a5b92))

### ❤️ Contributors

- kiki-kanri

## v0.2.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.1.0...v0.2.0)

### 🚀 Enhancements

- Add `init_tracing_with_local_time_format` utils and related dependencies ([5aff631](https://github.com/kikiutils/rust/commit/5aff631))

### 🎨 Styles

- Add `rustfmt.toml` and format code ([fd2104f](https://github.com/kikiutils/rust/commit/fd2104f))

### ❤️ Contributors

- kiki-kanri

## v0.1.0

[compare changes](https://github.com/kikiutils/rust/compare/28ddbea...v0.1.0)

### 🚀 Enhancements

- Add `task_manager` and related dependencies ([7575e22](https://github.com/kikiutils/rust/commit/7575e22))
- Add `wait_for_shutdown_signal` utils ([2382e23](https://github.com/kikiutils/rust/commit/2382e23))

### 🩹 Fixes

- Fix incorrect categories setting in `Cargo.toml` ([92f52f2](https://github.com/kikiutils/rust/commit/92f52f2))

### 🏡 Chore

- Add base files ([63a0732](https://github.com/kikiutils/rust/commit/63a0732))
- Add vscode settings file ([0271cad](https://github.com/kikiutils/rust/commit/0271cad))
- Set description field in `Cargo.toml` ([31746a3](https://github.com/kikiutils/rust/commit/31746a3))

### ❤️ Contributors

- kiki-kanri
