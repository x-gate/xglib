# Explicit Anime header layout

Date: 2026-09-14. Reference: [xgtool AnimeIndex.Load](https://github.com/x-gate/xgtool/blob/a5176dbf107f2f1567951476f652391b76f7f702/pkg/anime.go), treated as the compatibility reference at the user's request.

xgtool chooses a 12- or 20-byte header once at the AnimeInfo address, then uses that layout for every action. The original xglib entry point auto-detects each action independently. Two signed frame offsets of -1 can look like the extended sentinel in a later standard action, shifting the frame cursor and causing truncation errors.

The additive Rust API `Anime::build_from_bytes_with_header_size(info, data, header_size)` and WASM API `anime_build_from_bytes_with_header_size(info, data, header_size)` accept an explicit 12 or 20. Other values are errors. The explicit layout controls every action; extended sentinel/reserved values remain in the output even when a later sentinel is not -1. Input must still be the exact consumed record, with no trailing bytes.

`Anime::build_from_bytes` and `anime_build_from_bytes` retain their previous per-action detection, including mixed-layout synthetic cases. Existing consumers do not need migration. The explicit entry point returns the same Anime structure and signed fields; no generic palette or image semantics change.

rsc-manager detects the first layout, bounds-checks all action/frame lengths, and calls the new WASM function. It treats the next index address as a container boundary, passes only the span consumed by ActCnt actions, and reports any remaining container gap. No input bytes are edited or written back. Deployment order: build xglib WASM bindings, then rebuild rsc-manager; stale bindings do not have the new export.

Synthetic Rust regressions cover a later false sentinel, explicit extended layout with a noncanonical later sentinel, invalid header sizes, truncation, and trailing bytes. Existing Rust regressions remain unchanged. No original game data or xgtool implementation files were copied. Resource allocation limits remain the caller's responsibility; this addition is not a general parser-hardening change.

Validation for this change: 77 Rust tests and 5 Python verification-tool tests passed; rustfmt, Clippy with warnings denied, and cargo doc passed. The wasm32 release build and generated JS bindings passed rsc-manager's 21 WASM/frontend tests and 8 production-browser tests. No original-resource visual comparison was performed.
