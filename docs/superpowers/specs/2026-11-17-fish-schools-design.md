# Fish schools — Design

Part of issue #7 (the cove scene). Step 4 of the owner's priority order for the scene's look: waves,
lighting, kelp, **fish swarms**, creatures. Replaces the fish ported from the Swift scene (flat outlines in a
full-screen pass, rigid schools on a figure-of-eight). Builds on ADR 0009 (screens share the creatures, not
the motion).

## Intent

Echomere's cove is a meditative underwater webcam, 30 m down, looking 40° up at the surface, with a frame of
kelp at the left and right edges. The owner wants fish schools that behave like real ones: turns that run
through the school as a wave, a school that changes its shape (swimming in parallel, circling, a loose
cloud), splitting around something and closing up again, fish whose bodies swim. Calm, in keeping with the
slowed sea and kelp.

## Goals

1. **A big school as an event (owner's choice A)**: several hundred fish that visit every few minutes, enter
   the picture, roam close to the camera, then leave. Between visits the water is empty.
2. **One smaller school, more regularly (owner)**: about 30–60 fish, looser and quieter, in the picture more
   often and for longer.
3. **Real schooling behaviour**: each screen simulates its schools with the Couzin zone model (Couzin et al.
   2002: zones of repulsion, orientation and attraction, a blind angle behind each fish, a limited turning
   rate, noise). The school's shape follows from the zone widths (swarm, torus/milling, parallel group), and
   the school is steered through the picture by a few informed fish with a preferred direction (Couzin et
   al. 2005, leadership in groups).
4. **Fish with swimming bodies**: 3D fish, one procedural mesh per fish type drawn as instances, with a body
   wave in the vertex shader (the well-known sine-wave swim technique, as in ABZU's fish; Echomere's own
   code).
5. **Lit by the water model** like everything else; baitfish flanks reflect the light as they roll.
   Warmth at sunrise and sunset applies. (The first draft made a "silver cloud" the big school's look; that
   was the designer's idea, not the owner's, and is dropped: the goal is realistic behaviour.)
6. **Depth-correct**: fish are drawn with depth test, so the kelp frame covers fish behind it.

## Non-goals

- Whales (parked), echoes, kelp. Fish avoiding whales is designed in (below) but only matters once whales
  are back.
- Predators, feeding, bait balls, fast escape manoeuvres.
- More than two schools; other species.

## Anti-goals

- Hectic motion: darting, fast flickering tails, schools crossing the picture quickly. Everything is slow.
- Fish lost from the school, fish passing through each other, a school that never forms or never breaks
  formation.
- Fish swimming into the camera or crossing the kelp frame in front of the camera.
- A cost that spoils the energy goal: the simulation must stay a small fraction of the frame time.

## Constraints

- ADR 0009: what is shared between the screens of a cove is a pure function of (cove seed, visitors,
  echoes, time): **when** each school visits, its type and size, where it enters and leaves. The
  simulation of **how** it swims runs on each screen and may differ.
- Rust + wgpu + WGSL (ADR 0008); no platform branches in scene or shaders.
- Licences: the Couzin model is a published scientific model, implemented from the papers; no third-party
  code is copied. If any code is adapted after all, it is MIT/Apache only and credited in
  `THIRD-PARTY-NOTICES.md`.
- Render tests run on lavapipe; simulation tests run on the CPU, without a GPU.

## Approaches

The owner handed the how to the designer; the designer picked **1**.

1. **Couzin simulation on the CPU, fish drawn as instanced 3D meshes (chosen).** A few hundred fish are
   cheap on the CPU (neighbour search over a uniform grid), the behaviour is directly testable (alignment,
   milling, cohesion, spacing), and the renderer draws fish like the kelp blades.
2. Couzin on the GPU (compute shader): scales to thousands of fish, which isn't needed; harder to test; no
   faster at this size.
3. A closed-form, stateless school (lead path plus delayed followers): designed for pixel-identical screens,
   which ADR 0009 no longer requires; it imitates schooling instead of producing it.

## Decided

By the owner:

- A big school as a visiting event plus one smaller school that comes more regularly.
- Screens share the creatures (which schools, when), not the motion (ADR 0009).
- Calm, meditative.

Designer's calls, for the owner to check:

- **Fish types**: the big school is small silver baitfish (sardine-like, about 15–20 cm), the small school
  larger, darker fish (mackerel-like, about 30–40 cm).
- **Schedule (shared)**: `cove-scene` computes, from the cove seed and the time, the visits of each school:
  start, duration, type, size, entry and exit points and route. The big school starts every 5¼–9¾ minutes;
  the small school every 4–6 minutes. A visit lasts as long as its route takes at the lead speed (about 2–3
  minutes, plus 1–2 minutes of circling for some big-school visits), so the small school is in the water about
  half the time and the big one about a quarter. (The first draft's "every 2–4 minutes for 2–5 minutes" would
  have made the small school overlap itself.) Visits never depend on the frame before.
- **Simulation (per screen)**: a new crate `cove-swarm` (pure Rust, no GPU, stateful): Couzin zones with a
  blind angle and a maximum turning rate, a fixed time step with sub-steps, and a few informed fish whose
  preferred direction leads the school from its entry point, roaming around the camera (sometimes into a circling
  phase overhead, by widening the orientation zone over time), to its exit point. Schools spawn outside the
  picture when a visit starts and are removed after they have left it. Speeds are slow (see Time step and pace).
- **Space** (owner, 2026-11-18: closer): schools roam close to the camera, about 6–9 m ahead on its view axis,
  wandering up to 14 m to either side, so they leave the picture and come back; between
  the camera depth and a few metres below the surface. They avoid a sphere around the camera and, once
  whales are back, a capsule around each whale.
- **Rendering**: the full-screen fish pass is replaced by an instanced, depth-tested pass in `cove-render`,
  drawn after the kelp. Fish meshes are generated in code (no assets). The vertex shader bends each fish
  with a body wave whose speed follows the fish's swimming speed; the fragment uses the water model, a
  silver glint for baitfish, and `through_water`.
- **Shared route, local swimming**: each visit carries a route that is a pure function of time: a lead point
  moving from the entry point to a roaming phase and then to the exit point. While roaming (owner,
  2026-11-18: "it should linger a bit (more erratic paths)") the lead moves from stop to stop, 4–14 m apart,
  eased, and lingers 4–12 s at each; in circling visits it lingers 40–70 s at one stop while the school mills
  around it (the circling window). Every screen's informed fish steer towards
  this lead point, so the school is roughly in the same place on every screen while its shape and the
  individual fish differ (ADR 0009).
- **Starting mid-visit**: when a screen starts, wakes or is asked for an arbitrary moment (`--at`,
  snapshots) during a visit, it spawns the school as a loose parallel group around the route's lead point at
  that moment, heading along the route, and simulates a short warm-up (about 3 s of simulated time, in
  sub-steps) before the first frame. No visit is skipped, and nothing is fast-forwarded from the visit's start.
- **Time step and pace**: the simulation takes the real time since the last frame (a gap over 0.25 s, such
  as after sleep, is a new start, as above) and integrates in fixed sub-steps of 1/30 s. The pace is set by
  slow swimming speeds (baitfish 1.1 m/s, mackerel 1.3 m/s, the route's lead point 0.9 m/s), tuned with the
  owner, rather than by slowing time: the route is shared in real time, so the fish must keep up with it.
- **World frame**: fish positions are in the scene's world frame, the same as `WhalePose` and the kelp (x
  right, y up with the surface at 0, z ahead, camera at (0, −30, 0), pitched 40° up). "In the picture" means
  `camera::project` returns x and y in 0..1; entry and exit points lie outside the picture for every
  screen shape from 9:16 to 21:9 (checked at both ends).
- **Draw order**: the water pass (surface, shafts, echoes; full-screen, no depth write) first, then kelp,
  whales and fish, all with depth test `Less` and depth write. Fish stay at least 2 m below the surface;
  they need not avoid the kelp, the depth test sorts out which is in front.
- **Two schools at once**: the big and the small school can overlap in time. They are simulated together:
  social rules (orientation, attraction) apply only within a school, the repulsion zone applies to every
  fish, so the schools pass by or through each other without fish overlapping.
- **Capacity and preview**: at most 512 fish instances. `scene-preview` gets `--school big|small` to force a
  visit at the chosen moment, for snapshots, clips and tuning; tests use the `cove-swarm` API directly.
- **Energy**: the simulation runs at the frame rate (15 fps), and its CPU time is measured by a test with a
  budget (see Checks). Instances are uploaded once per frame like the kelp's.
- **Replaced**: `FishSchool`, `FishMember`, `CoveLayout.schools`, `fish_poses`, the creature list in the scene
  buffer and `fish.wgsl`; the old fish checks are replaced by the ones below.

## Checks

Simulation (`cove-swarm`, CPU):

- `school_forms_parallel_group`: with a wide orientation zone, after a settling time, the polarisation (mean
  of unit headings) is above 0.8.
- `school_mills_in_torus`: with the circling parameters, the milling order parameter (mean angular momentum
  around the centre) is above 0.6 and polarisation low.
- `school_stays_together`: no fish more than a set distance from the school's centre after settling; the
  school does not split permanently.
- `fish_keep_their_distance`: no two fish closer than half a body length.
- `informed_fish_lead_the_school`: the school's centre moves towards the preferred direction.
- `simulation_fits_the_budget`: one frame's simulation of 400 + 60 fish takes under 2 ms in a release build
  (skipped in debug builds).

Schedule (`cove-scene`):

- `visits_are_shared`: the same cove seed and time give the same visits; different coves differ.
- `routes_enter_and_leave_out_of_sight`: every route's entry and exit lie outside the picture at 9:16 and 21:9.
- `school_starts_mid_visit`: a school spawned in the middle of a visit is formed (polarisation above 0.8)
  and near the route's lead point after its warm-up.
- `big_school_visits_every_few_minutes` and `small_school_visits_more_often`: gaps and durations within the
  ranges above, over a simulated day.

Render (`cove-render`):

- `fish_are_drawn` (replaced): a frame with a school in view differs from one without.
- `kelp_covers_fish_behind_it`: a fish placed behind a blade does not change the blade's pixels.
- `night_fish_are_not_red` stays (adapted to the new pass).

## Left to the builder

- The Couzin parameters (zone radii, blind angle, turning rate, noise, speeds) and how the circling phase is
  driven, within the behaviour the checks pin.
- The neighbour search structure, the integration step and sub-steps.
- The fish meshes and body-wave constants, glint and colours, tuned from snapshots and a clip with the owner.
- How the simulation state lives in the renderer or the preview (who owns it, how it is reset when a visit
  starts).
