# zWipeRoll roadmap

This records the existing scope agreed in the project conversation; no repository roadmap was present. Completion requires executable checks and relevant interactive validation, not a build alone.

## 1. Real global scan — complete

- [x] Select detected drive-letter volumes and custom locations instead of scanning only standard picture folders.
- [x] Index image-containing folders recursively, deduplicate overlapping locations, and expose scan progress and read errors.
- [x] Cancel without replacing the existing catalog; scope resynchronization to selected locations and preserve unreadable folders and album progress.
- [x] Restore selected custom locations and handle a disconnected worker without leaving the UI running forever.
- [x] Regression tests, strict Clippy, debug and optimized release builds; focused code review.
- [x] Complete interactive scan/start/cancel/library/restart checks on the current executable.

Verification (2026-10-10): six tests passed and `cargo clippy --all-targets -- -D warnings` passed. Tests cover selected and overlapping roots, software-like album names, AppData exclusions, missing roots, cancellation, scoped resynchronization, decision undo and theme switching. Focused review checked worker completion/disconnection, cancellation and scoped catalog merging; no additional change was required.

Computer Use acceptance on `target/release/zwiperoll.exe`: deselecting all volumes disabled Start; adding the project assets location and scanning found exactly 2 images in 1 folder among 4 files, with 0 errors. Scanning the project and its overlapping assets root checked 7,608 files and still indexed only 2 images. An immediate start/cancel double-click stopped an in-progress run after 6,128 checked files with “Scan cancelled — previous index retained.” After closing and relaunching, both custom locations remained selected, the assets album was present, and existing library albums retained their displayed position, including Photo 127. No personal images were opened, reviewed or deleted. The project assets album remains in the local catalog as the acceptance sample. These are functional scan checks, not pixel-perfect acceptance or proof of all-disk coverage, exact-photo review continuation, or removable-volume reconnection.

## 2. Volume identity and offline libraries — implemented, hardware acceptance pending

- [x] Identify volumes independently of their drive letter and distinguish offline from removed locations without losing progress.
- [ ] Validate removable-volume disconnect/reconnect and changed drive letters.

Windows volume GUIDs now identify mounted volumes independently of letters, including folder-mounted volumes. The catalog remaps album, photo, session and selected-scan paths while preserving progress. Offline locations remain indexed; unavailable folders and reused-letter conflicts are blocked rather than interpreted as another disk. Missing folders on a mounted original volume are shown as unavailable, not silently erased. Scans discard results if the volume snapshot changes; Recycle Bin operations recheck identity before acting and retain failed entries.

Compatibility and limits: old catalogs have no historical volume identity, so their first upgrade can bind only currently mounted volumes; it cannot reconstruct a disk swap that happened before upgrading. A mount path reserved by a saved offline volume cannot be used for a different volume until the conflict is resolved. Network paths do not have a Windows volume GUID and retain path-based behavior. Snapshot checks are not an atomic operating-system lock against removal during a filesystem operation.

Verification (2026-10-10): 14 automated tests passed, strict Clippy passed, and the separately invoked read-only native Windows discovery probe passed. Automated acceptance covers disconnect/reconnect snapshots, changed/swapped letters, reused-letter collision chains, nested mounts, JSON compatibility, session progress and destructive-operation identity guards. Deliberately disabling collision protection and session remapping made their regression tests fail; restoring the fixes returned them to green. Independent review findings were corrected and re-reviewed without remaining blockers in that scope. Physical removable-disk acceptance remains UNVERIFIED and was explicitly deferred by the user; this milestone is not marked complete.

## Remaining agreed acceptance work

### Album continuation and ordering — automated acceptance complete

- [x] Persist real-folder and month-year album decisions and resume at the exact photo after restart and resynchronization.
- [x] Sort real folders by date or size in either direction; sort virtual months chronologically in either direction, including year boundaries.

Verification (2026-10-10): 29 tests passed, 0 failed, with the host-dependent native volume probe ignored; strict Clippy passed. Isolated on-disk restart tests exercise production persistence and continuation APIs without modifying user data. Reordering retains the current photo by path; if it was removed, continuation uses a clamped saved index. Added photos retain subsequent decisions, deleted reviewed photos no longer inflate counts, and undo updates real and virtual counts. Recursive parent/child overlaps preserve decisions and contribute each photo only once to virtual months. Continue selects the last virtual album rather than an unrelated real folder; an unavailable last month reports unavailability instead of silently switching albums.

Review findings in indexed reopening, removed-photo counts, and overlapping albums were fixed and independently re-reviewed without remaining blockers. Deliberately broken exact-photo selection, sorting, and continuation routing made their tests fail before restoration. Virtual months use local filesystem modification dates, not EXIF capture dates; undated photos remain available in real folders. This establishes automated functional acceptance, not a new interactive or pixel-perfect acceptance run.

Physical USB acceptance was explicitly skipped at the user's request. It remains UNVERIFIED, not passed, and does not block proceeding to the remaining UI work.

The optimized release build including album continuation and ordering passed (2026-10-10); `target/release/zwiperoll.exe` contains the updated implementation. No interactive, visual or physical-volume acceptance is implied by this build.

### Borderless window controls — automated checks passed, native acceptance pending

- [x] Route perimeter resizing and title/green-button actions through one native window-control implementation.
- [x] Verify all eight resize directions on primary press, title dragging, double-click and the rendered green sidebar control with headless input tests.
- [ ] Validate actual Windows drag, resize from every edge/corner, restoration of prior position/size, and maximized drag-to-restore interactively.

Verification (2026-10-10): 34 tests passed, 0 failed, with the host-dependent native volume probe ignored; strict Clippy passed. The native-perimeter regression failed against the previous inset hitboxes and passed after correction. Five new headless tests drive egui pointer events and inspect emitted viewport commands, including both observed maximize states and a click located from the rendered green circle. Deliberately disabling resize emission, click sensing and correct maximize toggling caused the expected failures; restoring the implementation returned the tests to green. Independent defect review found no remaining blocker in this scope.

Resizing now uses the viewport perimeter rather than layout margins and begins on the initial primary press. The reserved title strip does not overlap the resize perimeter or main content controls. Both title areas use click-aware native dragging; maximize/restore uses the operating system's saved geometry rather than application-issued size/position overrides. Native caption dragging is requested without an explicit unmaximize beforehand.

Limits: headless tests prove input routing and command generation, not the resulting HWND geometry or native mouse gesture. The user explicitly requested automatic checks only for this run; no app was opened and no mouse input was injected. Exact restoration and maximized drag-to-restore remain UNVERIFIED, so this acceptance item is not marked complete.

The optimized release build including these window-control changes passed (2026-10-10). The updated executable is `target/release/zwiperoll.exe`; it was not launched for this run.

### Visual acceptance — pending

- [ ] Verify all screens against the supplied light/dark references, full-logo placement, responsive layout, English text, and red-left/green-right review controls with Press A / Press D captions.

The remaining window and visual behaviors have inherited implementations; they are not marked complete without current acceptance evidence.

Visual reference measurements and outstanding comparisons are tracked in [visual parity acceptance](VISUAL_PARITY.md). Shared palette, control styling and vector review/folder icons have an implementation pass, but pixel-perfect acceptance remains open.

## 3. Video support — planned after the existing scan, library and UI acceptance work

Support mixed photo/video libraries using the same album and review workflow. This milestone is planned, not implemented.

- [ ] Index supported videos in single-folder scans, global scans, real-folder albums and virtual month-year albums; include them in date/size sorting.
- [ ] Show a thumbnail, duration and file size without starting playback automatically.
- [ ] Provide on-demand playback with play/pause, audio controls and seeking; keep the interface responsive during media loading.
- [ ] Reuse the red-left delete / green-right keep controls and Press A / Press D shortcuts. Playback shortcuts must not conflict with review decisions.
- [ ] Preserve review progress and resume at the exact video in its album after restart and resynchronization, consistent with photo continuation.
- [ ] Keep deletion deferred to the existing explicit Recycle Bin confirmation workflow.
- [ ] Choose and document the playback backend and actual supported container/codec combinations; report unsupported or damaged videos without blocking album navigation.
- [ ] Verify mixed-media albums, playback controls, unsupported files, persistence and deletion behavior with representative real videos; run regression checks for photo-only albums and both themes.

An indexed video is not necessarily playable: compatibility must be established from its container and codecs, not its filename extension alone. Resuming the album at a video does not imply restoring its playback timestamp in this first version.
