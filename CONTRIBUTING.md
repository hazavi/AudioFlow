# Contributing

Read [DEVELOPMENT.md](DEVELOPMENT.md) before changing native behavior. Use a normal Windows user account for development; the application should not require elevation.

Keep Windows operations in the Rust provider and worker. Keep routing decisions independent of COM so they can be tested with deterministic adapters. Add tests for changes to rule precedence, manual preference handling, ownership restoration, monitor selection, or update installation.

Before proposing a change, run the frontend build, frontend tests, Rust unit tests, and all-target Rust check. If routing changes, run the opt-in hardware test with two outputs and record whether live migration or stream recreation was needed. Mock tests do not establish hardware compatibility.

Do not introduce global audio-default changes, clear-all policy operations, browser-history collection, unverified update installers, telemetry, or signing secrets. A route preference that was accepted but has not moved an active stream must remain visibly pending.

Use small components, accessible control labels, the shared semantic tokens, and restrained motion. Test dark, light, and system modes. Explain unverified cases and error behavior in the change description.
