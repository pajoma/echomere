# Echomere

A meditative deep-sea screensaver where anonymous creatures travel from cove to cove, leaving echoes for strangers who never meet.

---

*Light ripples down from a surface far above. Kelp sways. Small fish drift past. Somewhere in the dim water, a whale you have never seen glides through your cove. It came from a place you may never visit, sent by someone you will never meet.*

---

## What it is

Echomere is a screensaver that looks into the deep sea, realistic, dark and slow, and uses hardly any energy. Each creator sends one whale on an endless trip. A cove is a town, shared by every creator who lives there, and every screen in it looks into the same water, like a webcam below the surface. A whale stays one day at each stop, leaves an echo when it moves on, and collects gestures (Bubbles, Song, Light) from the people who watched it. There are no accounts, no names, no free text and no moderation. Your whale might come home one day, but only by chance.

## Status

Design phase. There's no code yet.

- [VISION.md](VISION.md): the idea in three paragraphs
- [GLOSSARY.md](GLOSSARY.md): the domain terms (creature, creator, cove, echo, gesture, …)
- [docs/adr/](docs/adr/): architecture decisions
- [docs/vision/grilling.md](docs/vision/grilling.md): the vision interview and the confirmed summary
- [Issue #2](https://github.com/pajoma/echomere/issues/2): the version 1 spec

Version 1 is planned as a native macOS screensaver, a small web page for setup, travel logs and gestures, and a backend on Cloudflare Workers with D1 in an EU region.

## Contributing

Work happens on feature branches and reaches `develop` through pull requests. `main` is protected.

## Licence

[MIT](LICENSE).

Town data will come from [GeoNames](https://www.geonames.org/), licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

