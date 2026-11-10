# Echomere: Vision grilling

Interview questions for the vision and PRD, one round at a time. Each question has a recommendation (➡️).
Write your answer under **Answer:**. "yes" accepts the recommendation.

## Round 1: Root

### Q1: What is travelling_whales?

Describe it in one or two sentences: what someone does with it, and what they get out of it.
The name suggests real whales and their migrations. It could also be a metaphor (people who travel a lot,
"whale" customers) or just a codename.

➡️ It's a web app that shows the migration routes of individual whales, with sightings and tracking data on
a map. Anyone curious about whales can follow one whale on its journey.

**Answer:**
It is actually a screensaver showing the deep sea, it let's each user define his pet whales and maybe other animals (with name and origin and some other properties) which she can send on a trip (which actually means it shows on people's screens)

---

### Q2: What is it for?

- (a) a personal or hobby project, maybe to learn something
- (b) a product you want users or money from
- (c) something for a customer or your employer
- (d) open source or a public good, for example for researchers or conservation

This decides how hard we push on scope, quality and the business side later.

➡️ (a) It's a personal side project under `pajoma`. Mainly you want something fun that works, and learning
matters more than revenue.

**Answer:**
Just a hobby project, can be open source (maybe with a simple plugin system)

---

### Q3: Who uses it first?

Name the one main user for version 1. You could be the main user yourself. Other options are a member of the
public who likes whales, a whale-watching tourist, a marine researcher, or a teacher with a class.
If you name several, the next round asks until one comes first.

➡️ A member of the public who likes whales and wants to follow a whale's journey. Researchers and tour
operators can wait.

**Answer:**
People who want social screensaver, people who enjoy watching meditative backgrounds

---

### Q4: What makes you want to build it now?

Something started this: something that annoyed you, a dataset you found, an article, a trip, a technology you
want to try. The reason tells us what version 1 must get right.

➡️ No recommendation. Please give the real reason in a sentence.

**Answer:**
I thought about this years ago, but only now i can actually create this with claude

## Round 2: The screensaver and the trip

What Round 1 settled: travelling_whales is a social, meditative deep-sea screensaver. Each user owns whales
(and maybe other animals) with a name, an origin and other properties, and sends them on trips to other
people's screens. It's a hobby project, open source, maybe with plugins. You start it now because Claude makes
it buildable.

### Q5: Which platforms does version 1 run on?

A screensaver is tied to the operating system: macOS uses `.saver` bundles, Windows uses `.scr` files, and
Linux has xscreensaver or nothing. Options:

- (a) macOS only, the platform you work on
- (b) macOS and Windows
- (c) a scene that runs in the browser and fills the screen, shipped first as a web page and later wrapped in
  native screensaver shells for each OS
- (d) a desktop background (live wallpaper) instead of, or as well as, a screensaver

➡️ (c), with a macOS shell as the first wrapper. The scene runs in the browser, so one renderer works on every
platform, and the people a creature visits don't all need a Mac.

**Answer:**
a live wallpaper would be also great, but it definitly needs to be shell. Focus should be on resource minimazation, it should use hardly any energy is possible

---

### Q6: What do we call the animals a user owns?

You wrote "pet whales and maybe other animals". The glossary needs one word for "an animal that belongs to a
user and can travel". "Whale" doesn't fit once there are turtles or jellyfish. "Pet" sounds like you have to
look after it, and "animal" also covers the background fish nobody owns.

➡️ **Creature**: an animal a user owns and can send on trips. The unowned fish and plankton that fill the
scene are **ambient life**, and they never travel.


**Answer:**
creature sounds good, we focus on whales

---

### Q7: Whose screens can a creature visit?

- (a) anyone in the world running the screensaver, picked at random
- (b) only people you've connected with (friends, or a group you joined)
- (c) both: a trip to a group, or a trip "out into the open sea" to strangers

Strangers make the journey more magical. They also bring abuse (rude names), privacy questions and
moderation work, which is a lot for a hobby project.

➡️ (b) for version 1. A creature travels within a group of people who invited each other. (a) comes later,
once there's a way to moderate names.

**Answer:**
I want a "death stranding" feeling, there is no concept of users (with logins) and peers

I dont want moderation at all, maybe we can only focus on location (where does the creator live) and what kind of person this is (so no name)

---

### Q8: Is a creature on one screen at a time, or on many at once?

- (a) One at a time: it leaves your ocean, shows up on one screen, swims on to the next, and eventually comes
  home. This is a real journey with a route.
- (b) On many at once: a trip shows the creature on every recipient's screen, like a broadcast.

➡️ (a). The point is that it travels. A creature that's everywhere at once is just a notification. It also
means that while a creature is away, your own ocean is missing it, which gives the trip some weight.

**Answer:**
one at a time
---

### Q9: Do a creature's properties change what it does, or only what it shows?

You named name and origin. Other candidates are species, colour or pattern, size, age, a short message it
carries, and a travel log of the screens it has visited. Properties can be labels only (shown when it swims
by), or they can change its behaviour (speed, depth, how long it stays, where it goes next).

➡️ Mostly labels for version 1: name, species, origin (the owner's town or region, never an exact place) and
a travel log. Species alone decides how it looks and moves. The other properties don't change behaviour yet.

**Answer:**
the travel log is a neat idea. when a user starts, she needs to set her location in the browser. she can then see for her whale where it traveled (other peoples locations)

a user can always see the travel log of the whale currently visiting him

---

### Q10: What would a plugin add?

You wrote "maybe with a simple plugin system". Candidates:

- (a) new species: a model, its movement and its properties
- (b) new scenes: a coral reef or the Arctic instead of the deep sea
- (c) new behaviours: weather, day and night, events
- (d) all of these, through one extension format

➡️ (a) only, and not in version 1. The built-in species are written the way a plugin would be, so the plugin
format grows out of real use instead of being guessed up front.

**Answer:**
only a, coming later. 
## Round 3: Anonymous journeys

What Round 2 settled:

- The screensaver runs in native shells, and a live wallpaper would be nice too. Using hardly any energy is a
  top priority.
- The term is **Creature**, and the first version has whales only.
- There are no logins, no friends and no moderation. Strangers are connected the way they are in Death
  Stranding. A creature shows only where its creator lives and what kind of person they are. **It has no
  name** (Round 1 mentioned names; that's dropped).
- A creature is on one screen at a time.
- Each creature has a travel log. You set your location in the browser, and you can see where your whale has
  been and where the whale visiting you has been.
- Plugins come later and add new species only.

`GLOSSARY.md` now defines Creature, Ambient life, Trip and Travel log.

### Q11: What do we call the person and their installation?

Without accounts there are no "users". We need one word for the person behind a creature, and one for a
single installation of the screensaver: the place that has a location, where a creature lives and that other
creatures visit.

➡️ **Ocean**: one installation, with its location. A creature leaves its home ocean and visits other oceans.
**Keeper**: the person behind an ocean. Avoid user, owner, creator, host and peer.

**Answer:**
ocean sounds to big, keeper sounds like the pet, creator would be better i think
---

### Q12: How many creatures can one ocean have?

➡️ One whale per ocean. When it's away, your ocean has only ambient life and maybe a visiting whale. That
makes the trip feel like a real absence. Reinstalling doesn't give you a second whale on top of the first.

**Answer:**
only one 
---

### Q13: How does a keeper say "what kind of person" they are, without free text?

Free text needs moderation, so every choice comes from fixed lists. Options:

- (a) a few traits picked from a fixed list ("night owl", "sea lover", "parent", "student")
- (b) only the look of the whale (colour, pattern, scars), which the keeper picks to express themselves
- (c) both

➡️ (c). The keeper picks 2 or 3 traits from a fixed, translated list, plus the whale's look from fixed
options. Nothing anyone types ever reaches another screen.

**Answer:**
c
---

### Q14: How exact is the location?

The location goes to strangers' screens and into travel logs, so it's personal data under GDPR. Options:

- (a) the exact position from the browser
- (b) the town or region: the browser position is rounded on the keeper's device before it's sent anywhere
- (c) the country only

➡️ (b). The browser suggests a place, the keeper confirms or changes it, and only the rounded place (around
25 km) ever leaves the device. Travel logs show place names, never coordinates.

**Answer:**
b
---

### Q15: How does an ocean prove its creature is its own, without a login?

There are no accounts, but you still need to see your own whale's travel log, and nobody else should be able
to call your whale home.

➡️ On first start, the ocean generates a random secret key that stays on the device. If the keeper
reinstalls without a backup, the old whale is lost at sea and they start a new one. You can export the key
as a file if you want a backup.

**Answer:**
secret
---

### Q16: What does "it needs to be a shell" mean for rendering?

Using hardly any energy and sharing one scene across platforms pull in different directions:

- (a) a native renderer per platform (on macOS: Metal or SpriteKit), macOS first. This uses the least energy,
  but each platform needs its own renderer.
- (b) one web scene in a thin native shell (a WebView in a `.saver` bundle), with a capped frame rate. You
  write the scene once, but a WebView costs a lot more energy, which matters most for a wallpaper that runs
  all day.
- (c) native shells, with a small web page only for setup (location, traits, look) and for travel logs

➡️ (a) and (c) together. The scene is native, starting with macOS, and runs dark, slow and at a low frame
rate (around 15–20 fps). It pauses whenever the screen isn't showing it. A small web page does setup and
travel logs. The live wallpaper is the second shell, after the screensaver.

**Answer:**
a and c together
---

### Q17: How does a trip run?

Questions inside this one: who starts a trip, who picks where the creature goes next, how long it stays, and
when it comes home. There's also an edge case: the ocean it's visiting goes quiet for weeks because the
laptop is in a drawer.

➡️ The keeper sends the whale off. The next ocean is picked at random, with nearby oceans more likely, so
the route looks like a migration. The whale stays for about 30 minutes of that ocean's screensaver time, but
no longer than 24 hours in real time, so a quiet ocean can't hold it. After about 7 stops it heads home on
its own.

**Answer:**
random, but needs to be a neighbor
stays always 24 hours (one day), screensaver time does not matter
it will never head home, only by accident

---

### Q18: Can the person being visited do anything?

In Death Stranding you leave a "like", not a message. Options:

- (a) nothing: you only watch, and maybe open the travel log
- (b) one fixed gesture, such as "send a bubble", which appears in the travel log as "a keeper in Lisbon
  greeted you"
- (c) a small set of fixed gestures

➡️ (b). One gesture, no text. Your whale comes home with a log of the places that greeted it.

**Answer:**
c, which will show on the creator's screen and travel log
## Round 4: Where whales go, and what keeps them moving

What Round 3 settled:

- The person is the **Creator** (already in `GLOSSARY.md`). "Ocean" is too big a word, so the installation
  still needs a name.
- Each creator has one creature.
- "What kind of person" means fixed traits plus the whale's look. Locations are rounded to the town or region
  on the device. Ownership is a secret key on the device.
- Rendering is native per platform, macOS first. A small web page handles setup and travel logs.
- A trip is a random walk to a **neighbour**. Each stop lasts exactly 24 hours, however long the screensaver
  runs there. **The whale never heads home** on its own; it only comes back by chance.
- Visited people send gestures from a fixed set. Gestures show on the creator's screen and in the travel log.

### Q19: What do we call one installation?

It's the place with a rough location where the creature was born, and the place other creatures visit.
Candidates:

- **Cove**: small and sheltered, a place a whale passes through
- **Bay**
- **Waters**, as in "your waters"
- **Screen**: plain, but it names the device instead of the place

➡️ **Cove**. A whale leaves its home cove and wanders from cove to cove.

**Answer:**
cove is fine, but don't need to fix it, could also be bay, street of.. and so more (typical place names for oceanic regions)
---

### Q20: What counts as a neighbour?

Edge cases: a cove in Iceland has nobody within 500 km. A cove in Berlin has 400 others within 10 km. With
very few coves, a whale bounces back and forth between two of them.

- (a) every cove within a fixed radius (say 300 km)
- (b) the *k* nearest active coves, however far away they are
- (c) (b), but skipping the last few coves the whale visited, so it doesn't go back and forth

"Active" means the cove has been online in the last 7 days.

➡️ (c), with k = 8, skipping the last 5 coves visited. In crowded areas whales move slowly. In empty areas
they make long ocean crossings, which fits whales well.

**Answer:**
c is fine, but target needs to be empty
---

### Q21: How many visitors can one cove host at the same time?

With one whale per creator and 24-hour stops, there are about as many whales as coves. A popular cove could
fill up while a remote one stays empty for months.

➡️ At most 2 visiting whales at once. If every neighbour is full, the whale tries the next-nearest cove, and
if none is free it stays where it is for another day.

**Answer:**
yes
---

### Q22: What happens when a creator disappears?

Someone uninstalls, or their laptop dies. Their cove stops being a destination. What about their whale, which
is still out there?

- (a) It keeps swimming forever, even if nobody is left who cares about it
- (b) After 90 days without its creator going online, it dives deep and is gone. The last travel log entry
  says so.
- (c) It stays out there, but as a ghost whale: paler, with no creator traits

➡️ (b). This keeps the sea from filling up with orphans, and it's a quiet, fitting end.

**Answer:**
a, there are no orphans, but echos 
---

### Q23: Which gestures are there, and how do they reach the creator?

They have to be a fixed set with no text. A proposal:

- **Bubbles**: "someone in Lisbon greeted your whale"
- **Song**: a short whale call
- **Light**: a soft glow, for "I watched it for a long time"

On the creator's screen, a gesture is a small, slow effect, for example bubbles rising, with the place name
fading in and out.

➡️ These three, delivered no more than one per hour so a screen never gets busy.

**Answer:**
yes, all three
---

### Q24: What does a creator see at home, when their whale is (almost) never there?

- (a) Only ambient life and visiting whales. Your own whale exists only in the travel log on the web page.
- (b) Like (a), but with a faint line in a corner, such as "your whale: near Porto, day 43"
- (c) Like (a), but each new day your whale's arrival somewhere is shown as a short distant shadow

➡️ (b). One quiet line of text, shown now and then and never all the time.

**Answer:**
b

---

### Q25: Who runs the server, and on what?

Matching neighbours, handing whales over and keeping travel logs need a backend. It's a hobby project, it's
open source, and all data (rounded locations) is personal data, so GDPR applies.

- (a) a small VPS with SQLite that you run yourself (around €5 a month, plus maintenance)
- (b) serverless: Cloudflare Workers with D1, or a similar free tier (nothing to maintain, but you're tied to
  one provider)
- (c) no server: peer to peer between coves. This is complicated, and it's hard to make it use little energy.

Clients check in rarely: on start and then once an hour, because a whale changes cove only once a day.

➡️ (b), in an EU region, kept small enough that anyone can run their own instance from the open-source
code.

**Answer:**
b sounds fine (on cloudflare)
## Round 5: Gridlock, echoes and the first version

What Round 4 settled:

- **Cove** is the working term. Coves could also carry place names such as "Bay of …" or "Strait of …".
- A neighbour is one of the *k* nearest active coves, skipping the ones visited last. **The target cove must
  be empty.**
- Whales of vanished creators keep swimming forever as **echoes**. Nothing is ever an orphan.
- There are three gestures: Bubbles, Song and Light, at most one per hour.
- At home you see one quiet line about where your whale is.
- The backend runs on Cloudflare.

`GLOSSARY.md` now defines Cove, Neighbour and Gesture.

### Q26: One visitor per cove, or two? And how do we avoid gridlock?

Your answers conflict: Q20 says "the target needs to be empty" (one visitor), but you accepted two visitors
in Q21.

There's also a counting problem. Every cove has one creature, and echoes swim forever, so over time there are
**more whales than active coves**. With one visitor per cove, they can't all fit. Whales get stuck, and in a
full region nothing moves at all.

- (a) One visitor per cove. A whale with no empty neighbour waits another day. In a full region whales stay
  stuck until a cove frees up.
- (b) One visitor per cove, but **echoes don't take a place**: they drift through faintly and can share a
  cove with a visitor.
- (c) Two visitors per cove (Q21).

➡️ (b). A whale that still has a creator always has a cove to itself, and the growing number of echoes never
blocks the sea.

**Answer:**

---

### Q27: What exactly is an echo?

Questions inside this one: when does a whale become an echo (the creator's cove has been inactive for 90
days?), how does it look different, does it still collect a travel log, and what happens to gestures sent to
it, since nobody receives them.

➡️ A whale becomes an **echo** when its creator's cove has been inactive for 90 days. It turns paler and a
little translucent. Its travel log keeps growing, and gestures are recorded in the log but reach no one. If
the creator comes back with their secret key, the echo turns solid again.

**Answer:**
the gestures are still logged. an inactive account simply means that the creator is not looking at her creature anymore, but the creators live is not bound to his creator. a creature always has a home cove, regardless of the creator activity
---

### Q28: Do coves get oceanic place names?

Example: a cove in Lisbon becomes "Bay of Lisbon", "Strait of Lisbon" or "Lisbon Deep". The name is picked
once from a fixed set of patterns and shown in travel logs and gestures instead of the bare town.

➡️ Yes. The cove gets a name from a fixed set of patterns when it's created, and the name never changes. The
glossary term stays **Cove**.

**Answer:**
yes, for the pattern, but also works for landlocked locations, eg. munich bay
---

### Q29: Where does the visited person send a gesture from?

A screensaver closes at the first key press or mouse movement, and you can't click on a live wallpaper either.
So gestures can't be sent from the scene itself.

- (a) from the web page: "who is visiting me", with the visitor's travel log and three gesture buttons
- (b) from a tiny menu bar item on macOS, with the same content
- (c) both

➡️ (a) for version 1, and (b) later. A menu bar app runs all the time and adds an extra app to build and
sign.

**Answer:**
a, the screensaver always needs to show the url where to send feedback for the current creature
---

### Q30: How does the screensaver connect to the web page without a login?

The secret key lives on the device, but setup (location, traits, look) and travel logs happen in the browser.

➡️ In System Settings, the screensaver's **Options…** button opens the web page with a link that contains
the key in the URL fragment (`#…`), so the key never reaches the server's logs. The browser asks for the
location and rounds it before saving. You can bookmark the page to come back to your travel log.

**Answer:**
yes, through bookmark

---

### Q31: How is the macOS screensaver distributed?

For a `.saver` bundle to open without frightening warnings, it must be signed and notarized. That requires an
Apple Developer account (99 USD a year).

- (a) Sign and notarize with a developer account, and download it from GitHub Releases
- (b) Leave it unsigned, with instructions to allow it manually in System Settings
- (c) (b) at first, (a) once people actually use it

➡️ (c). There's no cost while it's just you and a few friends, and you sign it once strangers install it.

**Answer:**
c

---

### Q32: Which open-source licence?

➡️ **MIT**. It's simple and permissive, and it's compatible with whatever plugins come later. GPL would force
plugin authors to use the same licence.

**Answer:**
MIT is fine
---

### Q33: What's in version 1, and what isn't?

Proposal:

- **In**: a macOS screensaver (native, low energy), whales only, the web page (setup, travel logs, gestures),
  a Cloudflare backend, trips, neighbours, echoes, three gestures, cove names
- **Later**: live wallpaper, Windows and Linux, a menu bar item, plugins and other species, signing

➡️ Yes, exactly this cut.

**Answer:**
yes
## Round 6: Last open branches

What Round 5 settled:

- A creature's life isn't tied to its creator. It always keeps its home cove and keeps travelling, and
  gestures are logged even when nobody is looking.
- Coves get oceanic place names, including inland ones ("Munich Bay").
- Gestures are sent from the web page. **The screensaver always shows the URL** for sending a gesture to the
  creature currently visiting.
- The web page is reached through a bookmarked link that contains the key. The screensaver ships unsigned
  first and is signed later. The licence is MIT. The version 1 cut is confirmed.

Q26 had no answer, so it comes back here in a new form.

### Q34: What is an echo, then?

In Round 4 you wrote "there are no orphans, but echoes". In Q27 you said the whale isn't affected when its
creator stops looking. So an echo isn't a pale whale. What is it?

- (a) the trace a whale leaves in a cove after it moves on: a faint shimmer or a distant call in the next
  days, recorded in the travel log
- (b) a cove whose creator has stopped looking: it still exists and is still the whale's home, but nobody
  sees what happens there
- (c) something else

➡️ (a). It fits the Death Stranding feeling: you see traces of people you'll never meet.

**Answer:**
a
---

### Q35 (was Q26): How many whales fit in a cove, and what happens when the sea is full?

Coves whose creator stopped looking are no longer active, so they aren't destinations. Their whales keep
swimming, though. Over time there are **more whales than active coves**. If a target must be empty (your
Q20 answer), whales get stuck and full regions stop moving.

- (a) One whale per cove, strictly. A whale with no empty neighbour waits another day. It may wait a long time.
- (b) One whale per cove, but if no neighbour is empty, the whale may move to an occupied cove. That cove then
  has two whales for that day.
- (c) Whales whose creator stopped looking more than 90 days ago may also visit **inactive** coves. Nobody
  sees them there, but the travel log records it ("a quiet cove"), and this frees space in active coves.

➡️ (b) + (c). One whale per cove is the rule, two is the exception, and nothing blocks.

**Answer:**
we can have up to 5 whales in a cove (override previous decisions)
abandoned coves can be visited if overcrowded
but there a potentially more users then coves. e.g. the "munich bay" might be the starting cove of many whales. a whale can always travel to the deep sea, of there is space left. a shared starting location is visible for all users who set this as a start

---

### Q36: What does the gesture URL on screen look like?

The URL is visible to anyone who sees the screen, including colleagues walking past or a photo posted online.

- (a) one fixed URL per creature (anyone who ever saw it can keep sending gestures)
- (b) a short code that changes with each visit, such as `whales.example/k7m2`, valid only while the whale is
  in this cove
- (c) (b) plus a small QR code

➡️ (b). It's short, quiet text in a corner, so the screen stays calm. The code expires after the 24-hour
stop, so old photos are useless. One gesture per hour still applies.

**Answer:**
b
## Round 7: The cove is a shared place

What Round 6 settled:

- An **echo** is the trace a whale leaves in a cove after it moves on (now in `GLOSSARY.md`).
- Up to **5 whales** fit in one cove. This overrides "the target must be empty" (Q20) and "two visitors"
  (Q21).
- Abandoned coves can be visited when everything nearby is crowded.
- The gesture URL is a short code that changes with each visit.

Your Q35 answer also changes the model: **a cove is a place, not an installation.** Many creators in Munich
share "Munich Bay" as their home cove, and everyone whose home is Munich Bay sees the same visitors. I've
updated `GLOSSARY.md` to match. Some earlier decisions shift because of this, so here are the questions it
raises.

### Q37: A creature is in one cove at a time, but on many screens at once. Is that right?

In Q8 you chose "one screen at a time". Now a whale visiting Munich Bay shows on every screen whose home is
Munich Bay, maybe 50 of them, and it's in only one cove.

➡️ Yes. The rule is now "one cove at a time". Every screen in that cove sees the visitor, which makes a visit
something the neighbourhood shares.

**Answer:**
yes. the analogy of the screen is the webcam, many people can watch the webcam

---

### Q38: How are coves formed?

The browser position is rounded, but which cove does it belong to?

- (a) the nearest town from a fixed list of towns with more than about 15,000 inhabitants (a public dataset
  such as GeoNames). The cove is named after it: "Munich Bay", "Rosenheim Strait".
- (b) a grid of cells about 25 km across, each named after the biggest town inside it
- (c) the creator picks the town by hand from the list

➡️ (a), with the browser suggesting the nearest town and the creator able to pick another (c). Coves are then
predictable, privacy-friendly, and named after places people recognise.

**Answer:**
a
---

### Q39: What is the deep sea?

You wrote "a whale can always travel to the deep sea, if there is space left". Options:

- (a) a special cove with no place, where a whale goes when all its neighbours (including abandoned ones) are
  full. It has unlimited space, so it's always reachable, and it shows up in the travel log as "the deep
  sea".
- (b) the open water between coves. A long crossing takes several days and counts as travel time.
- (c) both: whales cross the deep sea between distant coves, and wait there when everything is full

➡️ (c). The deep sea is where whales are when they aren't in a cove: on long crossings, or when every cove is
full. Nobody sees them there, but the travel log notes "3 days in the deep sea".

**Answer:**
c
---

### Q40: With many viewers per cove, how do gestures reach the creator?

Fifty screens in Munich Bay see the same visitor. If each viewer sends a gesture, the creator gets 50 of them.

➡️ The travel log records every gesture, summed up per stop ("Munich Bay: 12 Bubbles, 3 Songs, 1 Light").
The creator's screen shows at most one gesture effect per hour, picking the most recent one.

**Answer:**
the gesture reaches the creator, not the home cove. it is in the creatures travel log. if he gets 30 gestures, than this is great and desired


## Round 8: Review findings

An architecture review of the summary above (Brooks-Lint, 2026-11-10) found gaps that would shape the data
model. These questions close them. The summary stays unconfirmed until this round is answered, and I'll
update it afterwards.

### Q41: Is every town a cove, or only towns where creators live?

There's one whale per creator, and a cove holds 5 whales. Many creators share a cove: "Munich Bay" might be
home to 400 of them, so at most 5 of those 400 whales can ever be there at once. The summary doesn't say
whether a town with no creators is a cove.

- (a) Every town on the list (GeoNames, more than about 15,000 inhabitants, around 30,000 worldwide) is a
  cove, watched or not. There's plenty of room, but most stops happen where nobody watches.
- (b) Only towns where at least one creator lives are coves. Every stop is watched, but dense regions are
  always full, and most whales wait in the deep sea, where nobody sees them.
- (c) Every town is a cove, but a whale prefers a **watched** cove (one with creators) as its next stop. It
  goes to an unwatched one only when the watched neighbours are full. The travel log calls that stop "a
  quiet cove".

➡️ (c). The sea never jams, most whales stay visible, and quiet coves replace "abandoned coves" as the
overflow. The deep sea stays for long crossings.

**Answer:**
c is good

---

### Q42: Shared scene, private overlay?

Every screen in a cove shows the same scene, like a webcam. But some things are for one creator only: the
line "your whale: near Porto, day 43" and the gestures their own whale received.

➡️ Yes, two layers. The **cove scene** is the same on every screen in that cove: terrain, ambient life,
visiting whales and echoes. On top of it, each screen draws its **creator's overlay**, which only that
screen shows: the home line, gesture replays, and the short code for sending a gesture.

**Answer:**
yes
---

### Q43: How literal is the webcam?

- (a) Same place, same visitors, but each screen moves its whales its own way. Two screens side by side look
  different.
- (b) Exactly the same picture: every screen computes the scene from the cove, the visitors and the current
  time, so two screens side by side show the same whale in the same spot. Nothing is streamed, so it doesn't
  cost extra energy.

➡️ (b). It keeps the webcam promise, and anyone who checks it with two laptops will find it true.

**Answer:**
a, the webcam is just an analogy, we  need to limit server load. Or can come up with an algorithm which makes location deterministic, depending on the time and a location seed? 

---

### Q44: How often do gestures show on the creator's screen?

You welcomed many gestures (Q40), but the screen should stay calm. 30 gestures replayed "a few times" each
means about 90 effects a day.

➡️ Gestures are bundled per stop: all gestures from Munich Bay become one effect, with bubbles, song and
light mixed according to what came in, and the cove name fading in. At most one effect every 30 minutes,
and each stop's bundle shows up to 3 times during the day it was received. The travel log still counts
every single gesture.

**Answer:**
90 effect a days is fine, max 1 effect per minute
---

### Q45: How often can one person send a gesture?

The short code is visible to anyone who sees the screen, and there are no accounts. One photo posted
online would let anyone flood a creator's travel log.

➡️ One gesture per code, per browser and per hour. Extra taps quietly do nothing. The limit is held on the
server, without storing who sent what.

**Answer:**
is fine
---

### Q46: Does the town lookup happen only in the browser?

To suggest the nearest town, either the browser sends its position to the server (which breaks "only the
cove leaves the device"), or the web page loads the whole town list and finds the nearest one itself
(around 1–2 MB, loaded once and cached).

Fact: GeoNames is licensed CC BY 4.0, so the web page and the README must credit it.

➡️ The browser does it. The town list is a static file on the web page, and coordinates never leave the
device.

**Answer:**
fine
---

### Q47: What does an echo look like, and how long does it last?

➡️ An echo is a faint shimmer, plus a distant call now and then, in the place where the whale swam. It
fades over 7 days. A cove shows at most its 5 most recent echoes, so a busy cove doesn't fill up with them.
The travel log notes "left an echo in Munich Bay".

**Answer:**
fine
---

### Q48: What happens when the key is lost?

Q15 suggested exporting the key as a backup file, but the summary dropped it. Without a backup, a reinstall
loses the whale for good.

- (a) That's intended: the whale is lost at sea, and the creator starts a new one.
- (b) The web page offers a backup: download the key as a small file, or save the bookmark somewhere safe.
  A lost key still means a lost whale.

➡️ (b). It costs almost nothing, and losing a whale you've followed for months to a laptop swap would feel
cruel.

**Answer:**
b
---

### Q49: How do we keep the macOS screensaver from wasting energy?

Research (2026-11-10, with sources) found that third-party `.saver` bundles on Sonoma, Sequoia and Tahoe keep
running and drawing after the screensaver is dismissed, and every start adds another copy. Apple has
acknowledged this and announced no fix. `stopAnimation` isn't called in normal use. On Tahoe, secondary
displays don't work with third-party screensavers at all.
(developer.apple.com/forums/thread/738547, /thread/787444; github.com/JohnCoates/Aerial/issues/1396)

Open-source screensavers such as Aerial work around it: they listen for the system's "screensaver will
stop" signal and quit their own process. That mostly works, but on Tahoe it wasn't always reliable.

- (a) Build a normal `.saver` bundle with these workarounds, starting from the open-source template
  ScreenSaverMinimal, and measure its energy use on Sonoma, Sequoia and Tahoe before committing to it.
- (b) Skip the `.saver` system: a small app of our own notices when the Mac is idle and shows the scene full
  screen itself. We control when it stops, but it doesn't appear under System Settings › Screen Saver, and
  it's one more app to install and run.
- (c) (a) first, and switch to (b) if the measurements show it still leaks.

➡️ (c). Build a throwaway prototype first that only measures energy use. ADR 2 waits for the result.

**Answer:**
c is good
---

### Q50: Where does the screensaver keep the secret key?

Every third-party screensaver shares one storage container on the Mac, so another installed screensaver
could read a key stored there as a plain file. Nobody has documented whether the macOS Keychain works from
inside a screensaver.

What the key protects: viewing your own whale's travel log and changing your cove, traits and look. It
doesn't touch money or private messages, and coordinates are never stored.

➡️ Use the Keychain if the prototype from Q49 shows it works. Otherwise use the shared container and accept
the risk, given what the key protects. Either way, the backup from Q48 stays possible.

**Answer:**
fine
## Round 9: Following up on Round 8

What Round 8 settled:

- Every town on the list is a cove. Whales prefer **watched coves** and go to **quiet coves** only when the
  watched ones nearby are full (now in `GLOSSARY.md`, replacing "abandoned cove").
- Each screen draws the shared cove scene and, on top of it, its own creator's overlay.
- Up to about 90 gesture effects a day are fine, at most one per minute.
- Gestures are limited to one per code, per browser and per hour. The town lookup happens only in the
  browser, and GeoNames gets credited.
- An echo fades over 7 days, and a cove shows at most 5. The web page offers a key backup.
- macOS: a `.saver` prototype that measures energy comes first, with our own idle app as the fallback. The
  key goes in the Keychain if that works, and in the shared container if not.

### Q51 (follows up on Q43): Is your idea the same as (b)?

You chose (a) to keep server load low, and asked about a deterministic algorithm based on time and a
location seed. That algorithm is (b): every screen computes where each whale is from the cove's seed, the
list of visitors and the clock. The server only says once a day who is visiting. Nothing is streamed, so the
server load is the same as with (a), and two screens side by side show the same picture.

➡️ Yes: deterministic from the cove seed, the visitors and the time, with no extra server load.

**Answer:**
if possible, yes
---

### Q52 (follows up on Q44): Does each gesture get its own effect?

"Max 1 effect per minute" sounds like you don't want gestures bundled per stop.

➡️ Yes. Each gesture is its own small effect, with the cove name fading in, shown up to 3 times on the day it
arrives and at most one effect per minute. If more arrive than fit, the oldest drop off the screen but stay
in the travel log.

**Answer:**
yes, 
---

### Q53: When does a cove count as quiet?

➡️ When no screen in that cove has checked in for 7 days. It's watched again as soon as one screen checks in.

**Answer:**
yes
---

### Q54: Where does a new whale start when its home cove is full?

Example: 400 creators live in Munich, and Munich Bay already holds 5 whales.

- (a) It starts in the deep sea and arrives in Munich Bay as soon as there's room.
- (b) It starts in the nearest watched cove that has room.
- (c) It always spends its first day at home, even if that puts a sixth whale in the cove. After that the
  normal rules apply.

➡️ (c). A creator should see their new whale at least once, and a sixth whale for one day doesn't jam
anything.

**Answer:**
c, the first day is only local
---

### Q55: When is a trip a deep-sea crossing, and how long does it take?

Now that every town is a cove, neighbours on land are close. Long crossings happen mostly over the sea, to
islands and between continents.

➡️ If the next cove is more than 500 km away, the whale crosses the deep sea first: one day for every 1,000
km, at least one day. If all 8 nearest coves (watched and quiet) are full, it waits one day in the deep sea
and then tries again.

**Answer:**
we have time, no need to rush

## Summary: what we agreed

Rounds 1–7 set the vision. Rounds 8 and 9 closed the gaps an architecture review found: which towns are
coves, what every screen shares, how often gestures show, and how the macOS screensaver stays frugal.

**Please check this summary.** Correct anything that's wrong. Once you confirm, the grilling is done. Two of
your answers I've interpreted; they're marked *(interpreted)*.

### Vision

Echomere is a meditative deep-sea screensaver. Each person creates one whale and sends it on an endless trip
from cove to cove around the world. People who never meet are connected through the whales that visit them,
the way strangers are in Death Stranding. There are no accounts, no names, no free text and no moderation.

### People and their creature

- A **creator** has exactly one **creature**. In version 1 it's always a whale, and it has no name.
- A creator is anonymous: what others see is their **cove** (a town) and 2 or 3 traits picked from a fixed
  list. The whale's look comes from fixed options too. Nothing anyone types ever reaches another screen.
- Ownership is a secret key on the device. A bookmarked link with the key in the URL fragment opens the web
  page. There are no logins. The web page offers a backup of the key, as a file or a saved bookmark; a lost
  key still means a lost whale.
- A creature lives independently of its creator. It keeps its home cove and keeps travelling even when the
  creator stops looking.

### Coves and screens

- **Every town** on a public list (GeoNames, more than about 15,000 inhabitants) is a **cove**, with an
  oceanic name: "Munich Bay", "Rosenheim Strait". The browser loads the list and suggests the nearest town
  itself, and the creator can pick another. Only the cove leaves the device, never coordinates. GeoNames is
  credited on the web page and in the README.
- A **watched cove** has at least one creator still looking. A cove becomes **quiet** when no screen in it
  has checked in for 7 days, and it's watched again as soon as one does.
- Many creators share a cove. A cove holds up to **5 whales**. **Ambient life** fills the scene.
- Each screen draws two layers:
  - the **cove scene**, the same on every screen in that cove: terrain, flora, ambient life, visiting whales
    and echoes. Each screen computes it from the cove's seed, the list of visitors and the clock, so two
    screens side by side show the same picture (if that turns out to be possible). The server only says once
    a day who is visiting; nothing is streamed.
  - the **creator's overlay**, which only that screen shows: the quiet line about where your own whale is
    ("near Porto, day 43"), gesture effects, and the short code for sending a gesture to the current visitor.

### Trips

- A **trip** never ends. Each stop lasts exactly 24 hours, however long anyone's screensaver runs.
- A new whale spends its first day at home, but only on its creator's screen, in the overlay. It doesn't take
  one of the cove's 5 places. *(interpreted from "the first day is only local")*
- The next stop is a random **neighbour**: one of the 8 nearest coves with room, skipping the last 5 visited.
  Watched coves come first; quiet coves count only when the watched ones nearby are full. The travel log
  calls such a stop "a quiet cove".
- If the next cove is more than 500 km away, the whale first crosses the **deep sea**: one day for every
  1,000 km, at least one day. If all 8 nearest coves are full, it waits a day in the deep sea and tries again.
  Nobody can see the deep sea. Slow, long crossings are fine. *(interpreted from "we have time, no need to
  rush")*
- A whale comes home only by chance.
- After a whale moves on, it leaves an **echo** in the cove it left: a faint shimmer and a distant call now
  and then. It fades over 7 days, and a cove shows at most its 5 most recent echoes.

### Gestures and travel logs

- There are three **gestures**: Bubbles, Song and Light. Anyone watching a visitor sends them from the web
  page, using the short code the screensaver shows (such as `echomere.example/k7m2`), valid only during this
  stop. Each browser can send one gesture per code per hour; the server enforces this without storing who
  sent what.
- Every gesture reaches the creature's **creator** and is recorded in the **travel log**. On the creator's
  screen, each gesture is its own small effect with the cove name fading in. It's shown up to 3 times on the
  day it arrives, at most one effect per minute. If more arrive than fit, the oldest drop off the screen but
  stay in the log.
- The travel log lists coves, quiet coves, days in the deep sea, echoes left and the gestures received. The
  creator can see it, and so can anyone watching while the whale visits.

### Look of the scene

- Realistic, not cartoonish or artsy, and always with low energy use in mind.
- A webcam placed below the surface: you still see the water surface above, with light playing through it.
- Each cove's scene is generated from its seed: terrain, flora such as kelp, and small animals (**ambient
  life**), so there's always some movement on screen.

### Technology and project

- The scene is a native renderer, macOS first. It's dark, slow and low frame rate, and it stops when nothing
  is showing it, so it uses hardly any energy.
- Third-party `.saver` bundles on current macOS keep running after the screensaver is dismissed. So the first
  step is a throwaway prototype: a `.saver` that quits its own process when the screensaver stops, with its
  energy use measured on Sonoma, Sequoia and Tahoe. If it still leaks, a small idle app of our own shows the
  scene instead. The secret key goes in the Keychain if the prototype shows that works, otherwise in the
  screensaver's shared storage.
- A small web page handles setup (cove, traits, look), the key backup, travel logs and gestures.
- The backend is Cloudflare Workers with D1 in an EU region, small enough for anyone to self-host. Clients
  check in rarely. Trip and scene logic take the time and a random seed as inputs, so they can be tested
  without waiting days.
- It's a hobby project under the MIT licence. It ships unsigned first, and is signed and notarized once
  strangers install it.

### Version 1

- **In**: the macOS screensaver (after the energy prototype), whales only, the web page, the Cloudflare
  backend, trips, neighbours, watched and quiet coves, the deep sea, echoes, three gestures, cove names
- **Later**: live wallpaper, Windows and Linux, a menu bar item, plugins (new species only), signing

**Confirmed:**
Confirmed