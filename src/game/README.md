# Game Engine

The game engine uses a loosely ECS (Entity-Component-System) architecture.

## Structure

- **Map** — the game world hierarchy (universe → world → dungeon → room)
- **Entities** — objects that exist within the map (players, NPCs, items, etc.)
- **Components** — data attached to entities, split into:
  - *Attributes* — state values (health, position, stats, etc.)
  - *Effects* — active modifications applied to an entity (buffs, debuffs, status conditions, etc.)
- **Systems** — logic that processes entities by reading and mutating their components each tick

## Design Notes

- Entities are identified by ID and located within the map hierarchy
- Components are pure data; behavior lives in systems
- Systems operate on all entities that have the relevant components
