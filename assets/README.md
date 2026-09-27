# App icon

`app-icon.png` is the original generated image. `app-icon.ico` packages the same
image at 16, 20, 24, 32, 40, 48, 64, 128 and 256 pixels for Windows. Pillow was
used only for resizing and ICO encoding. No content edits were made.

Generated with OpenAI image generation on 2026-09-27 for this project.
Design prompt: Create one final Windows utility app icon for Gear VR Controller.
Use a bold cyan circular touchpad disc and a white pointer at the lower right.
Use a Fluent style, a clear silhouette at small sizes, a transparent background
and about 12% outer margin. No text, brand logo, radio waves, shadows or multiple
alternatives. The icon represents daily computer control with an air mouse and
touchpad.

`build.rs` embeds the ICO as an EXE resource and copies it beside each build.
The window loads that copy. The tray uses the embedded resource. The package
script includes the ICO beside the executable.
