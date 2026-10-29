# Cursor Ring

The dot + trailing ring cursor from [zenolitee.github.io/portfolio-zen](https://zenolitee.github.io/portfolio-zen), for the whole Windows desktop.

It comes in two parts. You can use either one on its own:

- **The ring app** (`cursor-ring.exe`): a thin ring that follows the mouse, stretches with speed and turns gold over links. It runs from the tray.
- **The cursor scheme**: 17 matching Windows pointers (normal, link, text, busy, resize and the rest) that you install like any other scheme.

**[Open the editor in your browser](https://zenolitee.github.io/cursor-ring/tools/editor.html)** · **[Download the app and scheme](https://github.com/Zenolitee/cursor-ring/releases/latest)**

## Install the cursor scheme

1. Download `Cursor-Ring-scheme.zip` from the [latest release](https://github.com/Zenolitee/cursor-ring/releases/latest) and unzip it.
2. Right-click `install.inf` and choose **Install** (on Windows 11 it's under **Show more options**). Accept the admin prompt.
3. Open **Settings → Bluetooth & devices → Mouse → Additional mouse settings**, then go to the **Pointers** tab.
4. Choose **Cursor Ring** in the **Scheme** list and click **Apply**.

## Make your own version

Open the [online editor](https://zenolitee.github.io/cursor-ring/tools/editor.html) (or `tools/editor.html` locally), or click **Customize cursors...** in the ring app's tray menu. Change the colours, glow, outline, line thickness, dot and ring size, and spinner speed, and every cursor updates in a live preview. Hover a cursor to try it as your real pointer. **Download scheme** gives you a zip with the cursors and an `install.inf`. The editor page has a step-by-step guide.

## Run the ring app

Download `cursor-ring.exe` from the [latest release](https://github.com/Zenolitee/cursor-ring/releases/latest) and run it. Windows SmartScreen may warn about an unknown publisher, because the exe isn't code-signed: click **More info → Run anyway**.

To build it yourself, you need [Rust](https://rustup.rs).

```bash
cargo build --release
```

Then run `target\release\cursor-ring.exe`. The tray icon has **Ring settings...**, **Customize cursors...**, **Pause**, **Start with Windows** and **Quit**. While it's running, the normal arrow becomes the glowing dot. Pausing or quitting the app brings back your own scheme.

### Ring settings

**Ring settings...** in the tray opens `%APPDATA%\Cursor Ring\settings.ini` in Notepad. Change a value and save, and the ring updates straight away. You don't need to restart.

| Setting | Default | What it does |
| --- | --- | --- |
| `size`, `link_size` | 30, 42 | Ring width in px, normally and over links |
| `color`, `link_color` | #f0dcc0, #f0a45a | Ring colour, normally and over links |
| `opacity`, `link_opacity` | 0.75, 1 | 0 = invisible, 1 = solid |
| `thickness` | 1 | Line thickness in px |
| `follow` | 0.38 | Trail delay: 0.02 = long lazy trail, 1 = no delay |
| `stretch` | 0.5 | How much the ring stretches when moving fast (0 = never) |
| `dot` | true | Swap the arrow for the glowing dot. Set to false to keep your own pointer |

If a value is invalid, that setting falls back to its default. Delete the file to reset everything.

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
