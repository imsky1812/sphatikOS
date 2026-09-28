# Native app developer guide (draft)

> The SDK does not exist yet. This guide fixes the intended developer experience so the SDK can be built against it. APIs shown are provisional.

## Hello, Sphatik

```bash
sp new app --template single-view hello
cd hello && cargo run            # runs against the windowed compositor
```

`src/main.rs`:

```rust
use sphatik_ui::prelude::*;

fn main() -> sphatik_ui::Result<()> {
    App::new("dev.example.hello").run(|cx| {
        let count = cx.signal(0);
        Page::new("Hello")
            .large_title(true)
            .child(
                Column::new()
                    .gap(16)
                    .child(Text::new(move || format!("Tapped {} times", count.get())).style(TextStyle::Title2))
                    .child(Button::new("Tap me").style(ButtonStyle::Glass).on_press(move |_| count.update(|c| *c += 1))),
            )
    })
}
```

## The manifest

`Sphatik.toml` at the crate root:

```toml
[app]
id = "dev.example.hello"          # reverse domain, unique, never changes
name = "Hello"
version = "1.0.0"
icon = "res/icon"                 # folder with background, midground, foreground layers
categories = ["productivity"]

[permissions]
network = { reason = "Sync your notes" }
notifications = {}

[[intents]]
name = "note.create"
params = { title = "string", body = "string?" }

[[widgets]]
id = "recent"
sizes = ["small", "medium"]

[[live_activities]]
template = "progress"
```

## Packaging and installing

```bash
sp keygen ~/.sphatik/dev.key                 # once; keep this key safe, updates must use it
sp package --release --sign ~/.sphatik/dev.key
sp install target/hello-1.0.0.spk --device   # phone connected in developer mode
```

## Platform rules your app must follow

- Use the component library and design tokens; do not draw your own status bar or navigation bar.
- Put primary actions in the bottom 60% of the screen; bottom tab bars, not top tabs.
- Ask for permissions only when the user does the thing that needs them, and say why in `reason`.
- Keep the first frame under 500 ms: load data after the first frame, show placeholders.
- Support Dark mode, text sizes from 80% to 310%, and the screen reader (every control needs a label).
- Never block the UI thread for more than 8 ms.

Full design rules: [../design/hig.md](../design/hig.md). System APIs: [../api/sphatikd-dbus.md](../api/sphatikd-dbus.md).

## Publishing

Submit the signed `.spk` to the Sphatik Store with screenshots, a privacy summary (what data you collect and why) and a contact address. Review checks the manifest, the permissions against the stated reasons, and the HIG basics.
