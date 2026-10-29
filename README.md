# Cursor Ring

The dot + trailing ring cursor from [zenolitee.github.io/portfolio-zen](https://zenolitee.github.io/portfolio-zen), for the whole Windows desktop.

It comes in two parts. You can use either one on its own:

- **The ring app** (`cursor-ring.exe`): a thin ring that follows the mouse, stretches with speed and turns gold over links. It runs from the tray.
- **The cursor scheme**: 17 matching Windows pointers (normal, link, text, busy, resize and the rest) that you install like any other scheme.

## Install the cursor scheme

1. Download or clone this repo.
2. Right-click `assets/scheme/install.inf` and choose **Install** (on Windows 11 it's under **Show more options**). Accept the admin prompt.
3. Open **Settings → Bluetooth & devices → Mouse → Additional mouse settings**, then go to the **Pointers** tab.
4. Choose **Cursor Ring** in the **Scheme** list and click **Apply**.

## Make your own version

Open `tools/editor.html` in a browser, or click **Customize cursors...** in the ring app's tray menu. Change the colours, glow, outline, line thickness, dot and ring size, and spinner speed, and every cursor updates in a live preview. Hover a cursor to try it as your real pointer. **Download scheme** gives you a zip with the cursors and an `install.inf`. The editor page has a step-by-step guide.

## Run the ring app

You need [Rust](https://rustup.rs).

```bash
cargo build --release
```

Then run `target\release\cursor-ring.exe`. The tray icon has **Customize cursors...**, **Pause**, **Start with Windows** and **Quit**. While it's running, the normal arrow becomes the glowing dot. Pausing or quitting the app brings back your own scheme.

## Files

| Path | What it is |
| --- | --- |
| `src/main.rs` | Overlay window, tray menu and the cursor swap |
| `src/ring.rs` | Ring motion and drawing (`cargo test`) |
| `tools/editor.html` | Scheme editor (a single file, works offline) |
| `tools/make_scheme.py` | Rebuilds `assets/scheme` from the command line (`python tools/make_scheme.py`, standard library only) |
| `assets/scheme/` | The ready-made scheme and its `install.inf` |

## License

[MIT](LICENSE)
