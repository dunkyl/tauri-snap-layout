# Snap layout workaround for Tauri apps

See `src/snap_layout.rs` for most of the useful code. `use_snap_layout` is the where it gets started. It spawns a window to capture events and respond to `WM_NCHITTEST`. It sends some other events back through a channel for, in this case, hover and click events to be reflected in the webview 'manually'.

This repository uses dioxus, so to run it, it is easiest with their CLI.

```bash
dx serve
```

(If you juse use `cargo run`, it will not be styled.)
