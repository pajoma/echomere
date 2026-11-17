---
status: accepted
amends: 0005
---

# Screens of a cove share the creatures, not every frame

ADR 0005 made every screen in a cove compute the same picture from the cove's seed, the visitors and the
clock, like a webcam. That remains true for what the cove is about: which creatures are there, and where
the whales swim, so a creator sees their whale at the same place on every screen. It no longer holds for
how things move. Fish schools, kelp and a whale's body motion may differ from screen to screen. The owner
decided this (2026-11-17) because a realistic fish school is a simulation in which each fish reacts to its
neighbours: it carries state from frame to frame and is chaotic, so two machines would drift apart within
seconds anyway, and forcing a closed-form animation would make the schools less real. What is shared stays
a pure function of (cove seed, visitors, echoes, time): the whales' paths, and when which school visits.
What is animated on each screen only needs to look right, not identical.
