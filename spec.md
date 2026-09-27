# Ceramic Planet — POC Specification

## 1. Purpose

Build a small playable prototype for a cozy ceramics life-sim set on a tiny spherical planet.

The prototype should answer four questions:

1. Is walking around a genuinely spherical world pleasant and readable?
2. Does a rotatable top-down/isometric camera work well on that sphere?
3. Is the core loop of **explore → gather → craft → use/place** satisfying?
4. Does seeing crafted objects physically become part of NPC spaces make crafting feel more meaningful?

This is **not** intended to prove:

- a complete ceramics simulation
- a complex economy
- free-form object creation
- procedural planets
- advanced NPC AI
- realistic physics
- building construction
- combat
- survival mechanics

---

# 2. Core Fantasy

The player lives on a very small planet.

They own a primitive ceramics studio and spend their days:

- exploring the planet,
- finding clay and minerals,
- bringing resources back to their studio,
- making ceramic objects,
- selling or gifting objects,
- fulfilling requests from NPCs,
- improving their studio,
- watching objects created by themselves and NPCs gradually appear throughout the world.

The planet should feel like a **small physical place**, rather than an infinite game world.

---

# 3. Prototype Scope

## Included

### World
- One spherical planet.
- 3 simple geographical regions.
- Small enough to walk around completely.
- Resources respawn daily.
- Simple terrain variation.
- No loading screens between regions.

### Player
- Walk/run.
- Interact.
- Gather resources.
- Carry inventory.
- Operate machines.
- Craft ceramics.
- Place objects.
- Give objects to NPCs.
- Sell objects.

### Studio
Three machines:

1. Workbench / shaping station
2. Drying rack
3. Kiln

One upgrade for each machine.

### Ceramics
Three object families:

- cup
- bowl
- vase

Three material/glaze variations.

### NPCs
Three NPCs:

- Baker
- Carpenter
- Merchant

Each has:

- home or shop
- daily schedule
- workstation
- basic production
- relationship value
- one request chain
- one visible shop/home upgrade

### Persistent objects
Objects can physically appear:

- in the player's studio,
- outside in the world,
- inside/around NPC properties.

NPC properties use predefined placement slots.

### Time
- day/night clock
- NPC schedules
- machines use game time

---

# 4. Explicitly Out of Scope

Do NOT implement:

- arbitrary pottery sculpting
- pottery wheel simulation
- fluid simulation
- realistic clay deformation
- object shattering
- structural physics
- NPC pathfinding across the entire planet
- full simulated economy
- NPC-to-NPC trading simulation
- procedural towns
- procedural quests
- procedural planets
- seasons
- weather
- farming
- combat
- health
- stamina
- hunger
- skill trees
- dialogue generation
- complex character customization
- building construction
- multiplayer

If something isn't needed to validate the four prototype questions, fake it.

---

# 5. Visual Direction

Use simple primitives only.

## Player

Capsule or rounded rectangular body.

Example:

- capsule body
- sphere head
- two small cylinders for legs

No character animations required initially.

Walking can simply move the shape.

Later, a minimal bobbing animation can be added.

---

## NPCs

Use the same construction as the player.

Differentiate using:

- body color
- height
- head size
- simple accessory primitive

Examples:

Baker:
- white body
- small cylinder chef hat

Carpenter:
- brown body
- cube backpack/toolbox

Merchant:
- blue body
- small sphere hat

---

## Buildings

Simple combinations of:

- cubes
- rectangular prisms
- cylinders

Example bakery:

```text
      /\
     /  \
    /____\
   |      |
   |      |
   |______|
```

No detailed interiors required.

Buildings can initially have open fronts or removable roofs.

---

## Resources

Clay deposit:
- brown/red sphere cluster

Wood:
- cylinder trunk
- sphere canopy

Mineral:
- irregular-looking arrangement of small cubes

Plants:
- green capsules/cylinders

---

## Ceramic objects

Cup:
- cylinder

Bowl:
- flattened hemisphere

Vase:
- stacked cylinders/spheres

The POC does not need visually realistic pottery.

The important part is that the player recognizes:

> "This is the object I made."

---

# 6. Planet

## 6.1 Geometry

Use the cube-to-sphere approach described by Red Blob Games.

The world begins as six square grids representing the six faces of a cube.

Each face contains:

```text
N × N tiles
```

Recommended prototype resolution:

```text
24 × 24 tiles per cube face
```

Total:

```text
6 × 24 × 24
= 3,456 tiles
```

This is intentionally small.

---

# 6.2 Cube → Sphere

For each cube vertex:

```text
cube_position = vec3(x, y, z)
sphere_position = normalize(cube_position) * PLANET_RADIUS
```

This follows the basic cube-to-sphere projection described in the Red Blob exploration.

Recommended starting radius:

```text
Planet radius: ~40 world units
```

Experiment with:

```text
30
40
50
```

Do not optimize planet size yet.

Choose whichever produces the most pleasant visible curvature.

---

# 6.3 Terrain Height

After projecting onto the sphere:

```text
position =
    normal *
    (planet_radius + terrain_height)
```

Terrain variation should remain small.

Example:

```text
terrain_height = -0.5 → +1.5
```

This should create gentle hills rather than mountains.

Use:

- noise
- hand-painted height values
- or both

For the POC, hand-authored biome masks are preferred.

---

# 6.4 Tile Representation

Logical tile:

```text
PlanetTile {
    face
    x
    y

    biome
    height
    resource_type
}
```

The gameplay simulation remains tile-based even though the rendered surface is spherical.

This is important.

Do not base gameplay rules directly on arbitrary world-space coordinates.

---

# 6.5 Cube Face Transitions

The planet must support seamless movement across all six cube faces.

Red Blob separates the world coordinate into:

- cube face
- position within that face
- orientation

because crossing certain edges changes orientation.

Create a dedicated abstraction:

```text
PlanetCoordinate {
    face
    uv
    orientation
}
```

Do not scatter cube-transition logic throughout gameplay code.

All systems should use something like:

```text
planet.move_coordinate(coord, direction)
```

and let the planet system resolve:

```text
face A
    ↓
edge crossed
    ↓
face B
    ↓
coordinate transformed
    ↓
orientation adjusted
```

This will make later NPC navigation significantly easier.

---

# 7. Planet Regions

Use only three biomes.

## Meadow

Near the player's studio.

Resources:

- common clay
- grass/plants
- wood

Color:

```text
green
```

---

## Red Highlands

Resources:

- red clay
- iron mineral

Color:

```text
orange/red
```

---

## Coast

Resources:

- pale clay
- shells
- sand

Color:

```text
light beige
```

Water does not need simulation.

Represent ocean/coast areas using blue tiles.

Player cannot walk into deep water.

---

# 8. World Scale

The world should feel small.

Target:

```text
~3–5 minutes
```

to walk completely around the planet.

The player should quickly learn:

> "The red hills are behind the bakery."

> "The pale clay is past the merchant and around the coast."

This geographic familiarity is desirable.

Do not build an enormous exploration world.

---

# 9. Camera

The camera is one of the main experiments.

## Desired feeling

Combination of:

### Link's Awakening

- elevated
- top-down
- strong miniature/diorama presentation
- character remains highly readable

### Animal Crossing

- world visibly curves away
- environment appears to roll underneath the player
- world remains cozy rather than grand

But unlike either reference:

> The player can rotate the camera freely around their character.

---

# 10. Camera Rig

Use a pivot-based rig.

Hierarchy:

```text
Player
    ↓
Camera Target
    ↓
Camera Pivot
    ↓
Camera
```

Conceptually:

```text
             CAMERA
                \
                 \
                  \
              [pivot]
                 |
              PLAYER
                 |
            planet center
```

---

# 11. Camera Orientation

The camera's local up direction should follow the planet.

At the player's position:

```text
surface_up =
    normalize(player_position - planet_center)
```

The camera pivot should orient itself relative to this surface normal.

This prevents the planet from behaving like a flat map with curved visuals.

---

# 12. Camera Angle

Initial values:

```text
Pitch: 45–55°
Distance: 10–14 units
FOV: 25–35°
```

Start approximately with:

```text
Pitch: 50°
Distance: 12
FOV: 30°
```

A lower FOV helps create the miniature / diorama appearance.

Do not use a perfectly orthographic camera initially.

Perspective is useful for making the planet curvature visible.

---

# 13. Camera Rotation

Camera rotates around the player.

Input:

```text
Q / E
```

or

```text
Middle Mouse Drag
```

Later controller:

```text
Right Stick
```

Only yaw changes during normal rotation.

Example:

```text
camera_yaw += input * rotation_speed
```

Yaw rotates around:

```text
player_surface_normal
```

not global Y.

This distinction is essential on a sphere.

---

# 14. Camera Follow

Camera should smoothly follow the player.

Do not attach camera position directly to player position.

Use interpolation:

```text
camera_target =
    lerp(
        camera_target,
        player_position,
        follow_speed
    )
```

The camera should feel slightly soft.

Avoid excessive lag.

---

# 15. Camera and Planet Horizon

A key visual target is seeing objects disappear gradually around the curvature.

Example:

```text
Player
   ●

        tree
          ▲

             house
               █

-------------------------
       curved planet
```

As the player walks forward:

- terrain rises toward the player
- distant terrain disappears behind curvature
- new terrain appears ahead

This should happen naturally because the world is actually spherical.

Do not fake this using shader distortion in the POC.

---

# 16. Player Movement

Movement should be camera-relative.

Input:

```text
WASD
```

Determine:

```text
camera_forward
camera_right
```

Project both onto the tangent plane at the player's position.

Then:

```text
movement =
    camera_forward * input_y +
    camera_right * input_x
```

Normalize.

Move along the planet surface.

---

# 17. Staying on the Planet

After movement:

```text
normal =
    normalize(player_position - planet_center)
```

Place player at:

```text
normal *
(planet_radius + terrain_height + player_height)
```

Player orientation:

```text
player_up = normal
```

The character should therefore rotate naturally around the planet.

---

# 18. Physics

Keep physics extremely simple.

Needed:

- player/building collision
- player/resource collision
- interaction range
- object placement collision

Not needed:

- rigid-body ceramics
- realistic gravity
- breakage
- rolling objects
- stacking simulation

Placed objects are effectively static.

---

# 19. Interaction

Use one contextual interaction button.

Example:

```text
E
```

Mouse interaction may also be supported.

When near an interactable:

```text
[E] Gather clay
```

```text
[E] Use kiln
```

```text
[E] Talk to Mara
```

```text
[E] Pick up vase
```

Use whichever valid interactable is:

1. within range,
2. roughly in front of the player,
3. closest.

---

# 20. Inventory

Simple slot inventory.

Example:

```text
20 slots
```

Stackable resources:

```text
Clay x12
Iron x4
Wood x8
```

Ceramic objects occupy one slot each.

No weight system.

---

# 21. Resource Gathering

Resources exist physically on the planet.

Example:

```text
Clay Deposit
```

Interaction:

```text
E
```

Result:

```text
+3 Common Clay
```

The node visually disappears.

Respawns next day.

No tools required for the POC.

---

# 22. Ceramics Pipeline

Simplify ceramics to:

```text
RAW MATERIAL
     ↓
SHAPING
     ↓
GREENWARE
     ↓
DRYING
     ↓
DRY OBJECT
     ↓
FIRING
     ↓
CERAMIC OBJECT
```

Glazing can happen before firing for simplicity.

---

# 23. Workbench

Interact with workbench.

UI:

```text
MAKE

Cup
Bowl
Vase
```

Select:

```text
Bowl
```

Then select material:

```text
Common Clay
Red Clay
Pale Clay
```

Optional glaze:

```text
None
Blue
Green
White
```

Output:

```text
Unfired Bowl
```

No shaping minigame.

---

# 24. Drying Rack

Player places greenware onto rack.

Base rack:

```text
2 slots
```

Drying time:

```text
4 game hours
```

Upgrade:

```text
4 slots
```

Objects visibly sit on the rack.

---

# 25. Kiln

Base kiln:

```text
2 slots
```

Firing:

```text
4 game hours
```

Upgrade:

```text
4 slots
```

When firing completes:

```text
Dry Bowl
↓
Ceramic Bowl
```

---

# 26. Machine Upgrades

Each machine gets exactly one upgrade.

Example:

### Workbench

Level 1:
```text
Cup
Bowl
```

Level 2:
```text
Cup
Bowl
Vase
```

### Drying rack

Level 1:
```text
2 slots
```

Level 2:
```text
4 slots
```

### Kiln

Level 1:
```text
2 objects
```

Level 2:
```text
4 objects
```

Upgrades require:

```text
money + materials
```

No upgrade tree.

---

# 27. NPCs

Three NPCs.

## NPC 1 — Baker

Owns:

```text
Bakery
```

Produces:

```text
Bread
```

Wants:

```text
cups
plates/bowls
decorations
```

---

## NPC 2 — Carpenter

Owns:

```text
Workshop
```

Produces:

```text
Furniture
```

Can sell:

```text
shelves
studio upgrades
```

---

## NPC 3 — Merchant

Owns:

```text
General Store
```

Buys:

```text
ceramics
resources
```

Sells:

```text
basic materials
upgrade items
```

---

# 28. NPC Schedules

Schedules should be deterministic.

Example:

```text
06:00 Home
08:00 Walk to shop
09:00 Work
12:00 Outside / lunch
14:00 Work
18:00 Social location
21:00 Home
```

NPCs do not need sophisticated planning.

Schedule state machine:

```text
Home
↓
Travel
↓
Work
↓
Travel
↓
Social
↓
Travel
↓
Home
```

---

# 29. NPC Navigation

Do NOT create a general planetary AI-navigation solution initially.

Create predefined destinations and paths.

Example:

```text
BakerHouse
↓
Path01
↓
Bakery
↓
Path02
↓
TownSquare
```

NPC movement can follow waypoint curves projected onto the planet.

Later this can be replaced with planet-aware navigation.

---

# 30. NPC Production

NPC production is abstract.

Example:

```text
Baker Level 1
Produces:
3 Bread / day
```

There is no simulated:

```text
wheat
→ flour
→ dough
→ oven
→ bread
```

However, occasionally play visual animations:

```text
NPC walks near machine
machine emits particles
```

This creates the impression of work.

---

# 31. NPC Upgrades

Each NPC has one upgrade.

Example:

### Baker upgrade

Before:

```text
Small bakery
```

Request:

```text
4 ceramic cups
2 ceramic bowls
50 coins
```

After completion:

```text
Bakery Level 2
```

Visible changes:

- building gets larger,
- new table appears,
- player's cups appear on shelves,
- more NPCs visit it.

This is one of the POC's most important moments.

---

# 32. Relationships

Use one number:

```text
friendship: 0–100
```

Increase through:

```text
talking
+1

gift liked item
+5

complete request
+10
```

No romance.

No personality simulation.

No relationship decay.

---

# 33. Dialogue

Use authored dialogue.

Each NPC needs approximately:

```text
5 generic lines
3 request lines
3 relationship lines
```

Example:

```text
Baker:
"I keep running out of cups.
People apparently like drinking things."
```

Dialogue exists mainly to contextualize the world.

---

# 34. Object Persistence — Key Experiment

Ceramic objects should sometimes become persistent pieces of the environment.

This is a core feature of the prototype.

There are three object categories.

---

# 35. Category A — Inventory Objects

Exist only as data.

Example:

```text
Common Clay ×12
```

No world entity unless dropped.

---

# 36. Category B — Player-Placed Objects

Player can place:

- cup
- bowl
- vase

Placement mode:

```text
inventory
↓
select object
↓
preview ghost
↓
rotate
↓
place
```

Objects occupy real world space.

Placed objects remain where placed.

---

# 37. Category C — NPC/World Objects

NPC locations contain predefined slots.

Example bakery:

```text
BakeryPlacementSlots

counter:
    3

shelf:
    6

tables:
    8

window:
    2
```

When the player completes an order:

```text
4 Cups
```

some cups are assigned to those slots.

When visiting later:

```text
player sees their cups
```

This gives the illusion of a world slowly filling with created things.

---

# 38. Placement Slot

Conceptually:

```text
PlacementSlot {
    id
    position
    orientation
    allowed_categories
    occupied_by
}
```

Example:

```text
allowed_categories:
[
    Cup,
    Bowl,
    Vase
]
```

---

# 39. NPC-Created Objects

NPCs may also populate slots.

For example Carpenter:

```text
produces chair
```

After several days:

```text
chair appears outside bakery
```

Do NOT continuously spawn physical inventory.

Instead use events:

```text
WorldEvent:
    carpenter_added_bench
```

The object appears once.

---

# 40. Prevent World Clutter

Never implement:

```text
NPC makes 5 cups/day

100 days later:

500 cups physically exist
```

NPC production numbers are abstract.

Only meaningful objects become physical.

Rule:

> Physical objects represent consequences, not inventory counts.

---

# 41. Day Cycle

Suggested:

```text
1 real second = 6 game minutes
```

Therefore:

```text
10 real minutes ≈ 1 game day
```

The player can sleep early.

Sleeping:

```text
advances to next morning
```

At day transition:

- resources respawn,
- machines advance,
- NPC production updates,
- scheduled world events run.

---

# 42. Economy

Only one currency:

```text
Coins
```

Player earns coins from:

- selling ceramics,
- selling resources,
- completing requests.

Player spends coins on:

- machine upgrades,
- resources,
- studio upgrade.

No dynamic pricing.

---

# 43. Initial Progression

## Start

Player has:

```text
Workbench Lv1
Drying Rack Lv1
Kiln Lv1
```

Can create:

```text
Cup
Bowl
```

---

## First objective

Gather:

```text
Common Clay ×3
```

Create:

```text
Cup
```

Dry it.

Fire it.

Sell it.

---

## Second objective

Baker asks for:

```text
2 Cups
```

Player creates them.

Deliver.

Cups appear in bakery.

---

## Third objective

Explore Red Highlands.

Discover:

```text
Red Clay
```

Unlock:

```text
Vase
```

---

## Fourth objective

Upgrade kiln.

---

## Final POC objective

Baker asks for:

```text
4 Cups
2 Bowls
1 Vase
```

Completing request upgrades bakery.

The upgraded bakery visibly contains objects made by the player.

Prototype complete.

---

# 44. Studio

Player studio contains:

```text
Workbench

Drying Rack

Kiln

Storage

Placement Area
```

Studio should sit directly on the planet.

Avoid separate interior scenes initially.

Use an open workshop.

Example:

```text
        roof

   [Workbench]

[Rack]         [Kiln]

      Player
```

This eliminates:

- interior transitions,
- separate camera behavior,
- interior loading,
- interior coordinate systems.

---

# 45. Studio Upgrade

One overall upgrade is allowed.

Starting studio:

```text
small outdoor shed
```

Upgrade:

```text
larger workshop
```

Upgrade provides:

- additional machine space
- display shelf
- larger storage

Visually swap or add simple cube geometry.

---

# 46. Mouse Interaction

Because object manipulation will eventually be important, support mouse interaction early.

Mouse hover:

```text
raycast
↓
detect object
↓
highlight
```

Left click:

```text
interact/select
```

Right click:

```text
cancel
```

Mouse wheel or:

```text
Q/E
```

can rotate an object during placement.

Do not make the player's basic movement click-to-move in this POC.

Use WASD.

---

# 47. Camera Controls

Recommended controls:

```text
WASD
Move

Shift
Run

E
Interact

Left Mouse
Select / Interact

Right Mouse
Cancel

Middle Mouse Drag
Rotate Camera

Q / E
Rotate Camera

Mouse Wheel
Zoom
```

Zoom should be constrained:

```text
10–16 units
```

No completely free camera.

---

# 48. Camera Collision

For POC:

Do not implement complex camera collision.

Instead:

- buildings should be low,
- large objects can fade when between camera and player.

Simple rule:

```text
if object obstructs player:
    alpha = 0.3
```

Especially useful for trees.

---

# 49. Technical Structure — Bevy

Suggested high-level plugins/systems:

```text
Game
│
├── PlanetPlugin
├── PlayerPlugin
├── CameraPlugin
├── InteractionPlugin
├── InventoryPlugin
├── ResourcePlugin
├── CraftingPlugin
├── MachinePlugin
├── TimePlugin
├── NpcPlugin
├── PlacementPlugin
├── EconomyPlugin
└── SavePlugin
```

Avoid building a generic game framework.

Implement only what this POC needs.

---

# 50. Planet Components

Possible components/resources:

```text
Planet
PlanetTile
PlanetFace
SurfacePosition
SurfaceNormal
Biome
TerrainHeight
```

Characters should ideally have a planet-relative representation.

Example:

```text
SurfaceLocation {
    direction: Vec3,
    altitude: f32
}
```

World position:

```text
direction.normalize()
    * (planet_radius + altitude)
```

---

# 51. Surface Transform System

A reusable system should convert planet-relative data to transform.

Conceptually:

```text
surface_position
        ↓
sample terrain
        ↓
calculate normal
        ↓
world position
        ↓
orientation
```

Use this for:

- player
- NPCs
- resources
- buildings
- placed objects

This prevents each feature implementing spherical positioning differently.

---

# 52. Important Architectural Rule

Gameplay logic should think in terms of:

```text
planet surface
```

not:

```text
global X/Z plane
```

Avoid assumptions like:

```text
Vec3::Y == up
```

Instead:

```text
up =
normalize(position - planet_center)
```

This applies to:

- movement
- camera
- placement
- NPC orientation
- raycasts
- spawning
- buildings

---

# 53. Object Placement on Sphere

When placing an object:

Raycast from camera toward planet.

Get hit:

```text
hit_position
hit_normal
```

Object orientation:

```text
object_up = hit_normal
```

Rotate its forward direction around:

```text
hit_normal
```

Objects should therefore naturally follow the curvature.

---

# 54. Buildings

Buildings should also orient to surface normals.

However, because buildings occupy larger areas, keep them small relative to planet radius.

Do not deform buildings along curvature.

Treat each building as a rigid object sitting tangent to the sphere at its center point.

---

# 55. Save Data

Use simple serialization.

Save:

```text
player position
player inventory
coins
game time/day

machine upgrades
machine jobs

NPC friendship
NPC upgrade state
NPC request state

gathered resources

player placed objects
world placement slots
```

No sophisticated save architecture required.

One save slot.

---

# 56. Debug Tools

Very important for this POC.

Add debug toggles.

### Planet debug

Show:

```text
cube face colors
tile boundaries
surface normals
```

Each cube face should have a different debug color.

This makes seam problems obvious.

---

### Camera debug

Show:

```text
player normal
camera forward
camera right
movement vector
```

---

### NPC debug

Show:

```text
current schedule state
destination
path
```

Example above NPC:

```text
WORK → Bakery
```

---

# 57. Planet Test Mode

Before adding ceramics, build a dedicated planet test.

Spawn:

- player capsule
- colored planet
- random cubes
- trees
- six differently colored cube faces

Acceptance test:

Player must be able to:

```text
walk continuously
around planet
↓
cross every cube seam
↓
cross both poles
↓
rotate camera
↓
return to starting point
```

without:

- flipping controls,
- teleporting,
- sudden camera inversion,
- visible geometry cracks.

This milestone comes first.

---

# 58. Camera Test Mode

Populate planet with simple columns.

Test:

1. walk north
2. rotate camera 90°
3. walk east
4. cross cube seam
5. rotate 180°
6. cross pole
7. circle entire planet

At all times:

```text
W = visually forward
A = visually left
S = visually backward
D = visually right
```

Camera orientation must not corrupt movement.

---

# 59. POC Development Order

## Milestone 1 — Planet

Implement:

- cube grid
- cube-to-sphere conversion
- terrain mesh
- cube face transitions

No gameplay.

---

## Milestone 2 — Character

Implement:

- capsule player
- spherical movement
- surface orientation
- seam crossing

---

## Milestone 3 — Camera

Implement:

- elevated camera
- smooth follow
- yaw rotation
- zoom
- surface-relative orientation

At this point:

**STOP and evaluate whether simply walking around feels good.**

If the planet/camera combination is unpleasant, fix it before adding game systems.

---

## Milestone 4 — Exploration

Add:

- 3 biomes
- resource nodes
- gathering
- inventory

Loop:

```text
walk
→ find clay
→ gather clay
```

---

## Milestone 5 — Studio

Add:

```text
Workbench
Drying Rack
Kiln
```

Complete:

```text
Clay
→ Bowl
→ Dry Bowl
→ Fired Bowl
```

---

## Milestone 6 — Placement

Add:

- inventory selection
- world raycast
- placement preview
- rotate
- place
- pick back up

Now test:

> Is putting your creations physically into the world enjoyable?

---

## Milestone 7 — NPCs

Add three NPCs.

Implement:

- schedule
- simple waypoint movement
- dialogue
- shops
- relationships

---

## Milestone 8 — Persistent NPC Objects

Implement bakery slots.

Quest:

```text
Make 2 cups.
```

After delivery:

```text
cups physically appear
inside bakery.
```

This is the second major validation milestone.

---

## Milestone 9 — Progression

Add:

- machine upgrades
- NPC upgrade
- studio upgrade
- Red Highlands unlock/progression

---

## Milestone 10 — Save/Load

Persist everything necessary.

Run several game days.

Observe whether the planet gradually feels changed.

---

# 60. Target POC Play Session

A complete first playthrough should take roughly:

```text
30–60 minutes
```

Example:

```text
Start
 ↓
Learn movement
 ↓
Explore nearby meadow
 ↓
Gather clay
 ↓
Make first cup
 ↓
Dry + fire
 ↓
Meet baker
 ↓
Give baker cup
 ↓
Cup appears in bakery
 ↓
Explore coast/highlands
 ↓
Discover new material
 ↓
Make new ceramics
 ↓
Upgrade kiln
 ↓
Complete bakery order
 ↓
Bakery upgrades
 ↓
Player sees their ceramics
    incorporated into upgraded bakery
```

---

# 61. Success Criteria

The POC succeeds if testers say things equivalent to:

### Planet

> "I want to see what's around the other side."

### Camera

> "Moving around the little planet feels natural."

### Ceramics

> "I want to make another object."

### Persistence

> "That's the cup I made!"

### Social/world connection

> "My work is actually changing this place."

These reactions matter significantly more than:

- number of recipes,
- number of NPCs,
- realism of pottery,
- size of world,
- graphical quality.

---

# 62. Failure Signals

Reconsider the design if testers consistently say:

> "The spherical world makes me dizzy."

Then increase planet radius or reduce visible curvature.

---

If:

> "I don't care that my cup appeared in the bakery."

Then the persistent-object concept needs stronger contextual meaning.

---

If:

> "Crafting just feels like waiting."

Then add a small interaction to shaping/glazing rather than expanding the crafting system.

---

If:

> "I have no reason to explore."

Then materials need stronger geographical identities.

Do **not** solve these problems by adding more systems.

---

# 63. The Most Important Constraint

The POC should contain:

```text
1 planet

1 studio

3 machines

3 ceramic forms

3 material regions

3 NPCs

3 shops/workplaces

1 studio upgrade

1 upgrade per machine

1 upgrade per NPC location
```

Do not expand those numbers until the complete loop works.

---

# 64. Core Design Principle

Everything should reinforce:

> **I explore the planet to discover what it is made of, turn those materials into things, and those things gradually become part of the world.**

The planet provides the materials.

The studio transforms them.

The NPCs give them meaning.

Persistent objects record what happened.

That is the POC.