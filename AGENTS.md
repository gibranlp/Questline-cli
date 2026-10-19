# Project direction

- Questline is a local CLI application, including its integration into SpectrumOS. The hosted PHP API supports syncing between local installations.
- Web app development is on hold by the owner's explicit instruction. Do not develop, build, deploy, or expand `webapp/` unless the owner explicitly asks to resume that work. Preserve existing uncommitted web app changes unless asked otherwise.
- Focus current work on the CLI, local experience, SpectrumOS integration, and reliable sync/profile transfer.
- SpectrumOS Favorites belongs to the Questline profile as Portals. Shared links/categories/counts flow through the CLI's transactional encrypted sync; keep widget drafts, search, browser defaults and icon caches device-specific. See `docs/portals.md` for the interface and background service.
- SpectrumOS exposes Sync/profile controls through `sync-json`. Keep Transfer Codes out of command arguments and draft storage, require preview/confirmation for replacement or pruning, archive old widget drafts before rebinding, and preserve the profile binding checks. See `docs/sync-widget.md`.
- The PHP API deployment is authorized for the sync repair. Hold the CLI release until the owner specifies and completes the additional requested CLI feature; do not infer authorization to publish a CLI release from API deployment approval.

These directions were provided by the owner on 2026-10-19.

The owner authorized Questline 2.0.3 GitHub publication and SpectrumOS signed repository deployment on 2026-10-19. Web app development remains paused; this does not authorize an ISO build.
