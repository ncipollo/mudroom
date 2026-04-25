# Persona File Format

Persona files drive the behaviour of AI agent entities. Each file is a Markdown
document with three parts: YAML front matter, an optional preamble, and
conditional sections.

## Front Matter

The file opens with a YAML block delimited by `---`. The `name` and `role`
fields are the most important; any additional keys are stored as extra metadata.

```yaml
---
name: Bramble
role: innkeeper
alignment: neutral   # any extra key is preserved in `extra`
---
```

## Preamble

Text appearing before the first `#` heading is the preamble. It is always
included in the rendered instructions regardless of game state.

```markdown
You are Bramble, a warm and weathered innkeeper at the Rusty Flagon tavern.
You are gruff but fair, and speak in short sentences.
```

## Conditional Sections

Each H1 (`#`) heading starts a new section. A section may optionally begin with
a fenced YAML code block (tagged `yml` or `yaml`) declaring conditions. If all
conditions are met the section text is appended to the instructions; otherwise
the section is omitted. Sections with no conditions block are always included.

### Trust condition

Compares the current player's trust score (a numeric value tracked by the game)
against a threshold. Exactly one operator key must be present.

| Operator | Meaning              |
|----------|----------------------|
| `gt`     | greater than         |
| `gte`    | greater than or equal|
| `lt`     | less than            |
| `lte`    | less than or equal   |
| `eq`     | equal                |

```yaml
conditions:
  trust: { gt: 7 }
```

### Attribute condition

Compares a named entity attribute against a threshold. The `op` field accepts
the same operator strings as the trust shorthand above.

```yaml
conditions:
  attribute:
    name: threat_level
    op: gte
    value: 3
```

## Full Example

```markdown
---
name: Bramble
role: innkeeper
---

You are Bramble, a warm and weathered innkeeper at the Rusty Flagon tavern.

# Secrets

```yml
conditions:
  trust: { gt: 7 }
```

You guard a hidden cellar beneath the inn that stores smuggled goods.

# Quest Hook

```yml
conditions:
  trust: { gte: 5 }
```

A shipment of rare ale was stolen three nights ago. You suspect the miller.

# Combat

```yml
conditions:
  attribute:
    name: threat_level
    op: gte
    value: 3
```

Bramble will reach for the club kept under the bar if the player turns violent.

# Always Present

This section has no conditions block and is always appended to the instructions.
```
