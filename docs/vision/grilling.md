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


## Summary: what we agreed

Round 7 settled the rest: a screen is like a webcam on its cove, so many people watch the same visitor. Coves
are the nearest town from a public list. The deep sea covers both long crossings and waiting when everything
is full. Every gesture reaches the creator, and many gestures are welcome.

**Please check this summary.** Correct anything that's wrong. Once you confirm, the grilling is done.

### Vision

Echomere is a meditative deep-sea screensaver. Each person creates one whale and sends it on an
endless trip through coves around the world. People who never meet are connected through the whales that
visit them, the way strangers are in Death Stranding. There are no accounts, no names, no free text and no
moderation.

### People and their creature

- A **creator** has exactly one **creature**. In version 1 it's always a whale, and it has no name.
- A creator is anonymous: what others see is their **cove** (a town) and 2 or 3 traits picked from a fixed
  list. The whale's look comes from fixed options too. Nothing anyone types ever reaches another screen.
- Ownership is a secret key on the device. A bookmarked link with the key in the URL fragment opens the web
  page. There are no logins.
- A creature lives independently of its creator. It keeps its home cove and keeps travelling even when the
  creator stops looking.

### Coves and screens

- A **cove** is a town from a public list (GeoNames, more than about 15,000 inhabitants) with an oceanic name:
  "Munich Bay", "Rosenheim Strait". The browser suggests the nearest one, and the creator can pick another.
  Only the cove leaves the device, never coordinates.
- Many creators share a cove. Every screen whose home is that cove shows the same scene, **like a webcam**.
- A cove holds up to **5 whales**. **Ambient life** fills the scene.
- At home, a quiet line now and then says where your own whale is ("near Porto, day 43").

### Trips

- A **trip** never ends. Each stop lasts exactly 24 hours, however long anyone's screensaver runs.
- The next stop is a random **neighbour**: one of the 8 nearest coves with room, skipping the last 5
  visited. When those are full, abandoned coves count too. When even they are full, or the next cove is far
  away, the whale is in the **deep sea**, where nobody can see it.
- A whale comes home only by chance.
- After a whale moves on, it leaves an **echo** in the cove it left.

### Gestures and travel logs

- There are three **gestures**: Bubbles, Song and Light. Anyone watching a visitor sends them from the web
  page. The screensaver always shows a short code (such as `whales.example/k7m2`) that's valid only during
  this stop.
- Every gesture reaches the creature's **creator**. It's recorded in the **travel log** and shown on the
  creator's screen while the screensaver runs. Each gesture is replayed a few times during the same day, so
  the creator has a real chance to see it, but never so often that the screen gets busy.
- The travel log lists coves, days in the deep sea and the gestures received. The creator can see it, and so
  can anyone watching while the whale visits.

### Look of the scene

- Realistic, not cartoonish or artsy, and always with low energy use in mind.
- A webcam placed below the surface: you still see the water surface above, with light playing through it.
- Each cove's scene is generated once, when the cove is created: terrain, flora such as kelp, and small
  animals (**ambient life**), so there's always some movement on screen. Every screen in that cove shows the
  same scene.

### Technology and project

- The scene is a native renderer, macOS first. It's dark, slow and low frame rate, and it pauses when
  nothing is showing it, so it uses hardly any energy.
- A small web page handles setup (cove, traits, look), travel logs and gestures.
- The backend is Cloudflare Workers with D1 in an EU region, small enough for anyone to self-host. Clients
  check in rarely.
- It's a hobby project under the MIT licence. It ships unsigned first, and is signed and notarized once
  strangers install it.

### Version 1

- **In**: the macOS screensaver, whales only, the web page, the Cloudflare backend, trips, neighbours, the
  deep sea, echoes, three gestures, cove names
- **Later**: live wallpaper, Windows and Linux, a menu bar item, plugins (new species only), signing

**Confirmed:**
