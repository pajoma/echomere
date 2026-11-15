---
status: accepted
supersedes: 0002
---

# One renderer for macOS and Linux: Rust, wgpu and WGSL

Linux is a real platform next to macOS, not a later port. Rendering each platform natively (ADR 0002) would
mean writing and tuning every shader twice, in Metal and in a Linux API, for exactly the work that matters
most to the look. So the scene logic and the renderer are written once in Rust, drawn with wgpu (Metal on
macOS, Vulkan or OpenGL on Linux), with shaders in WGSL. wgpu is a native graphics library, not a web view,
so the reason ADR 0002 ruled out a shared web scene (a WebView running all day costs too much energy)
still holds. We chose Rust over Swift because wgpu is a Rust library with first-class support there, and
because the scene's checks then run on Linux as well as on the Mac. Each platform keeps a thin shell of its
own: a `.saver` bundle on macOS that calls the Rust library, and a fullscreen app on Linux that starts when
the computer is idle, since Wayland has no screensaver interface. The Swift and Metal scene (PR #10) becomes
the behavioural and visual reference for the port. The energy prototype from ADR 0002 (#3: does a `.saver`
quit when dismissed?) still applies to the macOS shell.
