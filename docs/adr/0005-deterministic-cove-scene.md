# The cove scene is computed on each screen, not streamed

Every screen in a cove shows the same scene, like a webcam. Instead of streaming positions from the server,
each screen computes the scene from the cove's seed, the day's list of visitors and the clock, so the server
only says once a day who is visiting. This keeps server load and energy use as low as letting each screen move
its whales its own way, while two screens side by side still show the same picture. What only one creator
should see (the line about their own whale, gesture effects, the short code) is a separate overlay on top of
the shared scene. Trip and scene logic take the time and a random seed as inputs, so they can be tested
without waiting days.
