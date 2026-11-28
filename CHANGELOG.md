# Changelog

## v1.0.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.14.0...v1.0.0)

### 💅 Refactors

- **logger:** Use std's default file buffer and document the console buffer size ([15d53bc](https://github.com/kikiutils/rust/commit/15d53bc))

### 📖 Documentation

- **global_allocator:** Remove module doc restating cfg selection ([63946b5](https://github.com/kikiutils/rust/commit/63946b5))
- Align rustdoc with logger and atomic cell behavior ([4668796](https://github.com/kikiutils/rust/commit/4668796))
- **logger:** Document wrapper-specific failure conditions ([4442531](https://github.com/kikiutils/rust/commit/4442531))
- Add README template ([8043c55](https://github.com/kikiutils/rust/commit/8043c55))
- Fold platform limits into the README summary ([68179ad](https://github.com/kikiutils/rust/commit/68179ad))
- **global-allocator:** Document per-platform allocator and manual installation ([8dff3eb](https://github.com/kikiutils/rust/commit/8dff3eb))
- Correct signal handler lifetime and clarify logger file routing ([4c919ef](https://github.com/kikiutils/rust/commit/4c919ef))

### 📦 Build

- **cargo:** Raise rust-version to 1.99 ([afc3fe9](https://github.com/kikiutils/rust/commit/afc3fe9))
- **cargo:** Anchor package include patterns to the crate root ([d4e5885](https://github.com/kikiutils/rust/commit/d4e5885))
- **deps:** ⚠️  Upgrade pathkit to v4 ([504430e](https://github.com/kikiutils/rust/commit/504430e))

### 🏡 Chore

- Update vscode configs ([31264ce](https://github.com/kikiutils/rust/commit/31264ce))

### ✅ Tests

- Cover worker batch limit and shutdown timeout ordering ([c26b0d4](https://github.com/kikiutils/rust/commit/c26b0d4))
- Tighten target sanitizing and timer panic assertions ([b3e189e](https://github.com/kikiutils/rust/commit/b3e189e))

### 🎨 Styles

- Tighten lint suppressions ([4a7b7ed](https://github.com/kikiutils/rust/commit/4a7b7ed))

#### ⚠️ Breaking Changes

- **deps:** ⚠️  Upgrade pathkit to v4 ([504430e](https://github.com/kikiutils/rust/commit/504430e))

### ❤️ Contributors

- Kiki-kanri

## v0.14.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.13.2...v0.14.0)

### 🚀 Enhancements

- **atomic:** ⚠️  Return typed transition errors and add try_update to AtomicEnumCell ([4ef7437](https://github.com/kikiutils/rust/commit/4ef7437))
- **logger:** ⚠️  Auto-detect console ANSI support ([9692689](https://github.com/kikiutils/rust/commit/9692689))
- **logger:** Add from_env_with_default_filter ([f44e9c6](https://github.com/kikiutils/rust/commit/f44e9c6))
- **logger:** ⚠️  Harden worker routing and filtering, reshape queue options ([50bbfa5](https://github.com/kikiutils/rust/commit/50bbfa5))
- **task:** Make TaskManager cloneable and add signal-driven cancellation ([e8d8aca](https://github.com/kikiutils/rust/commit/e8d8aca))
- **signal:** ⚠️  Report which shutdown signal was received ([cd8b40a](https://github.com/kikiutils/rust/commit/cd8b40a))
- **task:** Add spawn_critical ([fbabc86](https://github.com/kikiutils/rust/commit/fbabc86))
- **logger:** ⚠️  Add build_logger and timestamp options ([52b5d83](https://github.com/kikiutils/rust/commit/52b5d83))
- **service:** Return a typed StartError when start is refused ([733f52f](https://github.com/kikiutils/rust/commit/733f52f))
- **logger:** Cap queue capacity and split sinks into their own module ([67c0acf](https://github.com/kikiutils/rust/commit/67c0acf))
- **task:** Add TaskManager::shutdown_on_signal ([f032f3a](https://github.com/kikiutils/rust/commit/f032f3a))

### 🩹 Fixes

- **scripts:** Preserve cross-target imports during fixes ([c1964ed](https://github.com/kikiutils/rust/commit/c1964ed))
- **scripts:** Install Windows MSVC target before fix-windows runs ([d9e4c5d](https://github.com/kikiutils/rust/commit/d9e4c5d))
- **scripts:** Respect the min publish age when upgrading dependencies ([df0d370](https://github.com/kikiutils/rust/commit/df0d370))
- **logger:** Recompute the local offset each second so timestamps follow DST ([2ceddc6](https://github.com/kikiutils/rust/commit/2ceddc6))
- **logger:** Fail init before installing a subscriber and isolate console span fields ([72f4552](https://github.com/kikiutils/rust/commit/72f4552))

### 💅 Refactors

- Standardize module-qualified imports ([41bd7cb](https://github.com/kikiutils/rust/commit/41bd7cb))
- **logger:** Trim redundant docs and simplify timestamp formatting ([ad77dd9](https://github.com/kikiutils/rust/commit/ad77dd9))
- **logger:** ⚠️  Accept only canonical queue full policy names ([2e2c064](https://github.com/kikiutils/rust/commit/2e2c064))
- **logger:** ⚠️  Unify logger env var names ([180aa80](https://github.com/kikiutils/rust/commit/180aa80))
- **atomic:** ⚠️  Compare raw discriminants in ensure and make TransitionError non-exhaustive ([1a743d4](https://github.com/kikiutils/rust/commit/1a743d4))
- **task:** ⚠️  Rebuild TaskManager on TaskTracker with a reopenable root token ([b6f7191](https://github.com/kikiutils/rust/commit/b6f7191))
- **service:** ⚠️  Rebuild the async lifecycle on per-run tasks ([d1bd1d5](https://github.com/kikiutils/rust/commit/d1bd1d5))
- ⚠️  Remove the fx-collections module and feature ([d39178a](https://github.com/kikiutils/rust/commit/d39178a))
- **logger:** Simplify file sink and env parsing ([b8f1c6a](https://github.com/kikiutils/rust/commit/b8f1c6a))
- ⚠️  Remove the async-service module and feature ([a4fcb0a](https://github.com/kikiutils/rust/commit/a4fcb0a))
- **logger:** Reuse BoxedLayer and default write_notice to write_event ([6eeca0a](https://github.com/kikiutils/rust/commit/6eeca0a))

### 📖 Documentation

- Add logger file rotation todo ([22bb01d](https://github.com/kikiutils/rust/commit/22bb01d))
- Show feature requirements on docs.rs ([5bcea41](https://github.com/kikiutils/rust/commit/5bcea41))
- **atomic:** Document the AtomicEnumCell::try_update contract ([f0de698](https://github.com/kikiutils/rust/commit/f0de698))
- Drop Panics sections that only mention a missing time driver ([09b7fb4](https://github.com/kikiutils/rust/commit/09b7fb4))
- **todo:** Describe file rotation byte counting without the removed SinkFile ([5879105](https://github.com/kikiutils/rust/commit/5879105))
- **task:** Fold the shutdown_on_signal error condition into its summary ([c3e9e78](https://github.com/kikiutils/rust/commit/c3e9e78))

### 📦 Build

- **windows:** Install LLVM 23 from apt.llvm.org for xwin builds ([a7b0502](https://github.com/kikiutils/rust/commit/a7b0502))
- Cap dependency major versions and trim shared tokio features ([ac8a036](https://github.com/kikiutils/rust/commit/ac8a036))
- Enable tokio features per module and align dependency bounds ([cfbbb82](https://github.com/kikiutils/rust/commit/cfbbb82))
- Fix minimum versions of tracing and dev tokio ([aa75838](https://github.com/kikiutils/rust/commit/aa75838))
- **cargo:** Exclude dependency sources from coverage reports ([cc0ecfe](https://github.com/kikiutils/rust/commit/cc0ecfe))
- **cargo:** Keep only the full feature alias ([7343b2a](https://github.com/kikiutils/rust/commit/7343b2a))

### 🏡 Chore

- Allow unwrap and expect in tests via clippy config ([6102338](https://github.com/kikiutils/rust/commit/6102338))
- Update toolchain version ([f8cc420](https://github.com/kikiutils/rust/commit/f8cc420))
- Downgrade toolchain version ([9668a07](https://github.com/kikiutils/rust/commit/9668a07))
- Update public scripts and add RULES file ([a28bebf](https://github.com/kikiutils/rust/commit/a28bebf))
- Update ignore files ([6f3faa4](https://github.com/kikiutils/rust/commit/6f3faa4))
- **cargo:** Configure one-day publish age ([447f71d](https://github.com/kikiutils/rust/commit/447f71d))
- **rust:** Move to nightly-2026-11-14 and update Mordant ([6c9339a](https://github.com/kikiutils/rust/commit/6c9339a))
- Lint code ([fad1bc9](https://github.com/kikiutils/rust/commit/fad1bc9))

### ✅ Tests

- **service:** Merge the failing on_stop tests ([2242446](https://github.com/kikiutils/rust/commit/2242446))

### 🤖 CI

- Rewrite test and coverage workflow ([fb811a3](https://github.com/kikiutils/rust/commit/fb811a3))
- Add MSRV check job and macOS to the test matrix ([17610f9](https://github.com/kikiutils/rust/commit/17610f9))

#### ⚠️ Breaking Changes

- **atomic:** ⚠️  Return typed transition errors and add try_update to AtomicEnumCell ([4ef7437](https://github.com/kikiutils/rust/commit/4ef7437))
- **logger:** ⚠️  Auto-detect console ANSI support ([9692689](https://github.com/kikiutils/rust/commit/9692689))
- **logger:** ⚠️  Harden worker routing and filtering, reshape queue options ([50bbfa5](https://github.com/kikiutils/rust/commit/50bbfa5))
- **signal:** ⚠️  Report which shutdown signal was received ([cd8b40a](https://github.com/kikiutils/rust/commit/cd8b40a))
- **logger:** ⚠️  Add build_logger and timestamp options ([52b5d83](https://github.com/kikiutils/rust/commit/52b5d83))
- **logger:** ⚠️  Accept only canonical queue full policy names ([2e2c064](https://github.com/kikiutils/rust/commit/2e2c064))
- **logger:** ⚠️  Unify logger env var names ([180aa80](https://github.com/kikiutils/rust/commit/180aa80))
- **atomic:** ⚠️  Compare raw discriminants in ensure and make TransitionError non-exhaustive ([1a743d4](https://github.com/kikiutils/rust/commit/1a743d4))
- **task:** ⚠️  Rebuild TaskManager on TaskTracker with a reopenable root token ([b6f7191](https://github.com/kikiutils/rust/commit/b6f7191))
- **service:** ⚠️  Rebuild the async lifecycle on per-run tasks ([d1bd1d5](https://github.com/kikiutils/rust/commit/d1bd1d5))
- ⚠️  Remove the fx-collections module and feature ([d39178a](https://github.com/kikiutils/rust/commit/d39178a))
- ⚠️  Remove the async-service module and feature ([a4fcb0a](https://github.com/kikiutils/rust/commit/a4fcb0a))

### ❤️ Contributors

- Kiki-kanri

## v0.13.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.13.1...v0.13.2)

### 💅 Refactors

- Simplify logger enum parsing and utility implementations ([e757b91](https://github.com/kikiutils/rust/commit/e757b91))

### 📦 Build

- **deps:** Relax shared dependency version requirements ([010b9ff](https://github.com/kikiutils/rust/commit/010b9ff))

### ❤️ Contributors

- Kiki-kanri

## v0.13.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.13.0...v0.13.1)

### 🩹 Fixes

- Include toolchain libraries in fix script search path ([8b3e0d8](https://github.com/kikiutils/rust/commit/8b3e0d8))
- Include toolchain libraries in Windows MSVC environment ([f53a2c1](https://github.com/kikiutils/rust/commit/f53a2c1))

### 🏡 Chore

- Update config and toolchain version ([8e778dd](https://github.com/kikiutils/rust/commit/8e778dd))
- Update toolchain version ([919df37](https://github.com/kikiutils/rust/commit/919df37))
- Upgrade deps ([74147c0](https://github.com/kikiutils/rust/commit/74147c0))

### ❤️ Contributors

- Kiki-kanri

## v0.13.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.12.2...v0.13.0)

### 🚀 Enhancements

- Add Dylint setup and unified code-fix scripts ([e340201](https://github.com/kikiutils/rust/commit/e340201))
- **scripts:** Add Windows MSVC cross-target fix tooling ([95f8aaf](https://github.com/kikiutils/rust/commit/95f8aaf))
- **logger:** ⚠️  Make option structs non-exhaustive and add builder methods ([0f01aed](https://github.com/kikiutils/rust/commit/0f01aed))

### 🩹 Fixes

- **service:** Mark lifecycle as cleanup-failed when start or stop is cancelled ([f892c2e](https://github.com/kikiutils/rust/commit/f892c2e))

### 💅 Refactors

- **logger:** ⚠️  Rewrite logger for high-throughput non-blocking output ([71cca6f](https://github.com/kikiutils/rust/commit/71cca6f))
- **types:** Re-export FxHashMap and FxHashSet from rustc-hash ([4637b1f](https://github.com/kikiutils/rust/commit/4637b1f))
- **signal:** Simplify shutdown signal wait and cover Windows console events ([bd09638](https://github.com/kikiutils/rust/commit/bd09638))
- **task:** ⚠️  Remove registration lock and skip the calling task in existing-task operations ([2d83c01](https://github.com/kikiutils/rust/commit/2d83c01))
- ⚠️  Tighten feature deps, mark logger enums non-exhaustive and order service cleanup ([6c3b998](https://github.com/kikiutils/rust/commit/6c3b998))

### 🏡 Chore

- **lints:** Enforce cloning and test module conventions ([8d1aae7](https://github.com/kikiutils/rust/commit/8d1aae7))
- Lint and format code ([082f2ad](https://github.com/kikiutils/rust/commit/082f2ad))
- **dylint:** Track rules from the main branch ([f21c004](https://github.com/kikiutils/rust/commit/f21c004))
- Update mordant rules ([86817bf](https://github.com/kikiutils/rust/commit/86817bf))
- Lint code ([b1636af](https://github.com/kikiutils/rust/commit/b1636af))

#### ⚠️ Breaking Changes

- **logger:** ⚠️  Make option structs non-exhaustive and add builder methods ([0f01aed](https://github.com/kikiutils/rust/commit/0f01aed))
- **logger:** ⚠️  Rewrite logger for high-throughput non-blocking output ([71cca6f](https://github.com/kikiutils/rust/commit/71cca6f))
- **task:** ⚠️  Remove registration lock and skip the calling task in existing-task operations ([2d83c01](https://github.com/kikiutils/rust/commit/2d83c01))
- ⚠️  Tighten feature deps, mark logger enums non-exhaustive and order service cleanup ([6c3b998](https://github.com/kikiutils/rust/commit/6c3b998))

### ❤️ Contributors

- Kiki-kanri

## v0.12.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.12.1...v0.12.2)

### 🩹 Fixes

- **deps:** Relax minimum dependency patch versions ([799c85d](https://github.com/kikiutils/rust/commit/799c85d))

### ❤️ Contributors

- Kiki-kanri

## v0.12.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.12.0...v0.12.1)

### 🩹 Fixes

- Rename `move_to_sync` to `rename_to_sync` due to `pathkit` upgrade ([df87e61](https://github.com/kikiutils/rust/commit/df87e61))

### 💅 Refactors

- **task:** Adopt async closures and clarify cancellation token names ([5cbc2d6](https://github.com/kikiutils/rust/commit/5cbc2d6))

### 📦 Build

- **lint:** Migrate from Dylint to native Mordant ([e883385](https://github.com/kikiutils/rust/commit/e883385))

### 🏡 Chore

- Update `.gitignore` ([6e4ccf7](https://github.com/kikiutils/rust/commit/6e4ccf7))
- Update dylint mordant repo rev and bump toolchain channel to `nightly-2026-10-01` ([f74a335](https://github.com/kikiutils/rust/commit/f74a335))
- Update ignore files ([315edf8](https://github.com/kikiutils/rust/commit/315edf8))
- Upgrade deps ([34f3883](https://github.com/kikiutils/rust/commit/34f3883))
- Set mordant config ([fbd685d](https://github.com/kikiutils/rust/commit/fbd685d))
- Update `modify-files-permissions.sh` ([5fcc07d](https://github.com/kikiutils/rust/commit/5fcc07d))
- Upgrade deps ([e6f5577](https://github.com/kikiutils/rust/commit/e6f5577))
- Upgrade deps ([02714b1](https://github.com/kikiutils/rust/commit/02714b1))

### ❤️ Contributors

- Kiki-kanri

## v0.12.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.11.3...v0.12.0)

### 🚀 Enhancements

- **async-service:** ⚠️  Centralize cleanup in lifecycle hooks ([7f81129](https://github.com/kikiutils/rust/commit/7f81129))

### 💅 Refactors

- **task:** Rewrite task manager tracking ([10c4f64](https://github.com/kikiutils/rust/commit/10c4f64))

### 🏡 Chore

- Bump toolchain channel to `nightly-2026-08-01` ([30d3df8](https://github.com/kikiutils/rust/commit/30d3df8))
- Update `modify-files-permissions.sh` ([a004939](https://github.com/kikiutils/rust/commit/a004939))
- Update `modify-files-permissions.sh` ([347b3e4](https://github.com/kikiutils/rust/commit/347b3e4))
- Update `modify-files-permissions.sh` ([f7da945](https://github.com/kikiutils/rust/commit/f7da945))
- Update `.gitignore` ([1a910c9](https://github.com/kikiutils/rust/commit/1a910c9))
- Add dylint config and cargo alias ([1c18e2c](https://github.com/kikiutils/rust/commit/1c18e2c))
- Bump toolchain channel to `nightly-2026-09-01` ([505d781](https://github.com/kikiutils/rust/commit/505d781))
- Upgrade deps ([fe34e3d](https://github.com/kikiutils/rust/commit/fe34e3d))
- Add `clone_on_ref_ptr` lint rule ([595d64d](https://github.com/kikiutils/rust/commit/595d64d))

### 🎨 Styles

- Use explicit Arc::clone syntax ([069502e](https://github.com/kikiutils/rust/commit/069502e))

#### ⚠️ Breaking Changes

- **async-service:** ⚠️  Centralize cleanup in lifecycle hooks ([7f81129](https://github.com/kikiutils/rust/commit/7f81129))

### ❤️ Contributors

- Kiki-kanri

## v0.11.3

[compare changes](https://github.com/kikiutils/rust/compare/v0.11.2...v0.11.3)

### 🩹 Fixes

- **scripts:** Preserve wrapper script context ([bd61b95](https://github.com/kikiutils/rust/commit/bd61b95))

### 💅 Refactors

- Resolve clippy warnings ([daf207f](https://github.com/kikiutils/rust/commit/daf207f))

### 🏡 Chore

- Bump toolchain channel to `nightly-2026-07-01` ([5b78b02](https://github.com/kikiutils/rust/commit/5b78b02))
- Bump toolchain channel to `nightly-2026-07-15` ([9af4542](https://github.com/kikiutils/rust/commit/9af4542))
- Sync public script libs ([59c2783](https://github.com/kikiutils/rust/commit/59c2783))
- Upgrade deps ([54714f1](https://github.com/kikiutils/rust/commit/54714f1))
- Sync public scripts libs from upstream ([4d75fea](https://github.com/kikiutils/rust/commit/4d75fea))
- **lint:** Refine crate lint policy ([bb4980c](https://github.com/kikiutils/rust/commit/bb4980c))
- Ignore local AI tooling artifacts ([4b42b7c](https://github.com/kikiutils/rust/commit/4b42b7c))
- Upgrade deps ([08c81eb](https://github.com/kikiutils/rust/commit/08c81eb))

### ❤️ Contributors

- Kiki-kanri

## v0.11.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.11.1...v0.11.2)

### 🚀 Enhancements

- **service:** Add async service lifecycle ([83e814e](https://github.com/kikiutils/rust/commit/83e814e))

### 🏡 Chore

- Update example `Cargo.toml` ([0b12d63](https://github.com/kikiutils/rust/commit/0b12d63))

### ❤️ Contributors

- Kiki-kanri

## v0.11.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.11.0...v0.11.1)

### 📖 Documentation

- Correct `LoggerInitOptions::from_env` docs ([50cabff](https://github.com/kikiutils/rust/commit/50cabff))

### ❤️ Contributors

- Kiki-kanri

## v0.11.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.10.0...v0.11.0)

### 💅 Refactors

- ⚠️  Rename some logger env keys ([01b30f3](https://github.com/kikiutils/rust/commit/01b30f3))

#### ⚠️ Breaking Changes

- ⚠️  Rename some logger env keys ([01b30f3](https://github.com/kikiutils/rust/commit/01b30f3))

### ❤️ Contributors

- Kiki-kanri

## v0.10.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.9.0...v0.10.0)

### 🏡 Chore

- ⚠️  Rename `tracing` feature name to `logger` ([7c21f80](https://github.com/kikiutils/rust/commit/7c21f80))

#### ⚠️ Breaking Changes

- ⚠️  Rename `tracing` feature name to `logger` ([7c21f80](https://github.com/kikiutils/rust/commit/7c21f80))

### ❤️ Contributors

- Kiki-kanri

## v0.9.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.8.0...v0.9.0)

### 🚀 Enhancements

- **logger:** Add configurable tracing logger ([d2c90eb](https://github.com/kikiutils/rust/commit/d2c90eb))
- **logger:** Add env options builder ([99a5212](https://github.com/kikiutils/rust/commit/99a5212))
- **logger:** Add non-blocking log workers ([e65eb76](https://github.com/kikiutils/rust/commit/e65eb76))

### 🩹 Fixes

- Lint codes ([c7ce11d](https://github.com/kikiutils/rust/commit/c7ce11d))

### 💅 Refactors

- ⚠️  Remove `tracing` related components and codes ([898f80c](https://github.com/kikiutils/rust/commit/898f80c))
- **logger:** Use pathkit file moves ([4f8d841](https://github.com/kikiutils/rust/commit/4f8d841))

### 🏡 Chore

- Update `upgrade-dependencies.sh` script ([f1a2a24](https://github.com/kikiutils/rust/commit/f1a2a24))
- Update scripts ([85d43bd](https://github.com/kikiutils/rust/commit/85d43bd))
- Update fmt rules and set lint rules and update rust toolchain nightly date to 26-06-15 ([89d68c4](https://github.com/kikiutils/rust/commit/89d68c4))
- Update lint rules ([3cc9417](https://github.com/kikiutils/rust/commit/3cc9417))
- Update lint rules ([16550a8](https://github.com/kikiutils/rust/commit/16550a8))
- Fmt code ([aed428b](https://github.com/kikiutils/rust/commit/aed428b))
- Add `--message-format=short --quiet` args into lint and lint-fix command ([2f89d10](https://github.com/kikiutils/rust/commit/2f89d10))
- Update cargo alias commands ([e0cf8fe](https://github.com/kikiutils/rust/commit/e0cf8fe))

### ✅ Tests

- Add atomic and signal unit coverage ([e38e7d9](https://github.com/kikiutils/rust/commit/e38e7d9))
- **signal:** Support windows target checks ([b4addf4](https://github.com/kikiutils/rust/commit/b4addf4))
- Gate integration modules by feature ([3ee704c](https://github.com/kikiutils/rust/commit/3ee704c))

#### ⚠️ Breaking Changes

- ⚠️  Remove `tracing` related components and codes ([898f80c](https://github.com/kikiutils/rust/commit/898f80c))

### ❤️ Contributors

- Kiki-kanri

## v0.8.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.7.3...v0.8.0)

### 🚀 Enhancements

- **global-allocator:** Add platform allocator alias ([d75ebcd](https://github.com/kikiutils/rust/commit/d75ebcd))

### 🩹 Fixes

- **scripts:** Preserve entrypoint log prefixes ([b48c56c](https://github.com/kikiutils/rust/commit/b48c56c))
- **task:** Enable fx collections feature ([c972479](https://github.com/kikiutils/rust/commit/c972479))

### 💅 Refactors

- ⚠️  Remove `anyhow-ext` feat and related files ([22d342d](https://github.com/kikiutils/rust/commit/22d342d))
- **task:** ⚠️  Simplify lifecycle tracking ([63846e6](https://github.com/kikiutils/rust/commit/63846e6))

### 🏡 Chore

- Update release script ([ea640cc](https://github.com/kikiutils/rust/commit/ea640cc))
- Update cargo config ([330e4ed](https://github.com/kikiutils/rust/commit/330e4ed))
- Set build rustflags and release profile configs ([35839b5](https://github.com/kikiutils/rust/commit/35839b5))
- Add fmt script and update release script ([a956ad0](https://github.com/kikiutils/rust/commit/a956ad0))
- Add `rust-toolchain.toml` to use nightly version and update vscode settings ([6b261ba](https://github.com/kikiutils/rust/commit/6b261ba))
- Configure profile-specific `rustflags` in `cargo.toml` for dev and release ([0624e2f](https://github.com/kikiutils/rust/commit/0624e2f))
- Remove fmt script and update cargo alias ([2a9ad8c](https://github.com/kikiutils/rust/commit/2a9ad8c))
- Update `.cargo/config.toml`, add multi-platform example rustflags ([6d0c420](https://github.com/kikiutils/rust/commit/6d0c420))
- Add `.gitattributes` ([a7d2a67](https://github.com/kikiutils/rust/commit/a7d2a67))
- Update cargo config, add build release scripts and bump toolchain channel to 260515 ([510c6cf](https://github.com/kikiutils/rust/commit/510c6cf))
- Update scripts and configs ([fc8d7bb](https://github.com/kikiutils/rust/commit/fc8d7bb))
- Update release build scripts ([a2812b6](https://github.com/kikiutils/rust/commit/a2812b6))
- Update rust toolchain nightly date to 26-06-01 ([7858ac2](https://github.com/kikiutils/rust/commit/7858ac2))
- Add `.omx/` to `.gitignore` ([3f01e6d](https://github.com/kikiutils/rust/commit/3f01e6d))
- Add `upgrade-dependencies.sh` script ([7ab5176](https://github.com/kikiutils/rust/commit/7ab5176))
- Update `upgrade-dependencies.sh` script ([d7d3e79](https://github.com/kikiutils/rust/commit/d7d3e79))
- Upgrade deps ([1f61c6e](https://github.com/kikiutils/rust/commit/1f61c6e))
- Update `release.sh` ([f6892db](https://github.com/kikiutils/rust/commit/f6892db))

### 🤖 CI

- Rename `release-test-codecov.yaml` to `continuous-verification.yaml` ([d262250](https://github.com/kikiutils/rust/commit/d262250))
- Update `codecov/codecov-action` version to v6 ([9c3e6aa](https://github.com/kikiutils/rust/commit/9c3e6aa))
- Update `continuous-verification` workflow ([752c1fd](https://github.com/kikiutils/rust/commit/752c1fd))

#### ⚠️ Breaking Changes

- ⚠️  Remove `anyhow-ext` feat and related files ([22d342d](https://github.com/kikiutils/rust/commit/22d342d))
- **task:** ⚠️  Simplify lifecycle tracking ([63846e6](https://github.com/kikiutils/rust/commit/63846e6))

### ❤️ Contributors

- Kiki-kanri

## v0.7.3

[compare changes](https://github.com/kikiutils/rust/compare/v0.7.2...v0.7.3)

### 🏡 Chore

- Update deps ([38b4b70](https://github.com/kikiutils/rust/commit/38b4b70))
- Replace `DashMap` with `FxDashMap` type alias ([16443ab](https://github.com/kikiutils/rust/commit/16443ab))
- Update cargo alias ([3577a55](https://github.com/kikiutils/rust/commit/3577a55))
- Update vscode settings ([4db069d](https://github.com/kikiutils/rust/commit/4db069d))
- Update deps ([8d31826](https://github.com/kikiutils/rust/commit/8d31826))
- Update release script ([a8b2e40](https://github.com/kikiutils/rust/commit/a8b2e40))

### 🤖 CI

- Add test workflow template file ([305020b](https://github.com/kikiutils/rust/commit/305020b))

### ❤️ Contributors

- Kiki-kanri

## v0.7.2

[compare changes](https://github.com/kikiutils/rust/compare/v0.7.1...v0.7.2)

### 🏡 Chore

- Remove `num_enum` re-exports ([4e67da3](https://github.com/kikiutils/rust/commit/4e67da3))

### ❤️ Contributors

- Kiki-kanri

## v0.7.1

[compare changes](https://github.com/kikiutils/rust/compare/v0.7.0...v0.7.1)

### 🚀 Enhancements

- Add `num_enum_derive` crate and re-exports ([d02d814](https://github.com/kikiutils/rust/commit/d02d814))

### ❤️ Contributors

- Kiki-kanri

## v0.7.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.6.0...v0.7.0)

### 💅 Refactors

- ⚠️  Move `num_enum` re-exports to single file ([b4fbbd2](https://github.com/kikiutils/rust/commit/b4fbbd2))

#### ⚠️ Breaking Changes

- ⚠️  Move `num_enum` re-exports to single file ([b4fbbd2](https://github.com/kikiutils/rust/commit/b4fbbd2))

### ❤️ Contributors

- Kiki-kanri

## v0.6.0

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.11...v0.6.0)

### 🚀 Enhancements

- Add `IntoPrimitive` and `TryFromPrimitive` re-exports from `num_enum` crate ([c41a18a](https://github.com/kikiutils/rust/commit/c41a18a))
- Add `fx-collections` types ([a8f4ea8](https://github.com/kikiutils/rust/commit/a8f4ea8))

### 💅 Refactors

- ⚠️  Rename `AtomicEnum` to `AtomicEnumCell` ([18ce124](https://github.com/kikiutils/rust/commit/18ce124))

### 🏡 Chore

- Make `rustc-hash` crate optional ([ef7bb37](https://github.com/kikiutils/rust/commit/ef7bb37))

#### ⚠️ Breaking Changes

- ⚠️  Rename `AtomicEnum` to `AtomicEnumCell` ([18ce124](https://github.com/kikiutils/rust/commit/18ce124))

### ❤️ Contributors

- Kiki-kanri

## v0.5.11

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.10...v0.5.11)

### 🚀 Enhancements

- Add `AtomicEnum` struct ([8569432](https://github.com/kikiutils/rust/commit/8569432))

### ❤️ Contributors

- Kiki-kanri

## v0.5.10

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.9...v0.5.10)

### 🏡 Chore

- Fix decimal precision for seconds formatting in `make_tracing_fmt_layer_with_local_time` ([a08695f](https://github.com/kikiutils/rust/commit/a08695f))

### ❤️ Contributors

- Kiki-kanri

## v0.5.9

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.8...v0.5.9)

### 💅 Refactors

- Update `make_tracing_fmt_layer_with_local_time` to return a concrete type instead of using a trait ([b72f6fd](https://github.com/kikiutils/rust/commit/b72f6fd))

### 🏡 Chore

- Lint code ([95b7641](https://github.com/kikiutils/rust/commit/95b7641))

### ❤️ Contributors

- Kiki-kanri

## v0.5.8

[compare changes](https://github.com/kikiutils/rust/compare/v0.5.7...v0.5.8)

### 🚀 Enhancements

- Split `fmt` creation from `init_tracing_with_local_time_format` into separate function returning a layer; add `init_tracing_with_layer` and `make_tracing_fmt_layer_with_local_time` ([2d6c88d](https://github.com/kikiutils/rust/commit/2d6c88d))

### ❤️ Contributors

- Kiki-kanri

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
