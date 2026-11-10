---
status: proposed
---

# Native renderer per platform, macOS first, after an energy prototype

The scene is rendered natively on each platform (Metal on macOS first), not as a shared web scene in a
WebView, because using hardly any energy matters more than writing the scene once, and a WebView running all
day as a screensaver or wallpaper costs far more. Third-party `.saver` bundles on Sonoma, Sequoia and Tahoe
are known to keep running after the screensaver is dismissed (Apple developer forums threads 738547 and
787444, Aerial issue 1396). So a throwaway prototype comes first: a `.saver` that quits its own process when
the screensaver stops, measured on all three versions. If it still leaks, a small idle app of our own shows
the scene instead. This ADR becomes accepted once the prototype settles which.
