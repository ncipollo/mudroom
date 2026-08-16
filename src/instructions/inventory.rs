pub fn render() -> String {
    let sections = [
        header(),
        fields_section(),
        equipment_slots_section(),
        character_type_section(),
        example_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions inventory — inventory config file reference

inventory.toml lives at the top level of a mud's config directory
(e.g. muds/basic/inventory.toml) and defines the shape of
character-owned inventories: bag capacity and named equipment slots.
The file lists one or more named inventory configs, so a mud may
support more than one inventory shape even though most define just
one, named "inventory"."#
        .to_string()
}

fn fields_section() -> String {
    r#"Top-level shape — one or more [[inventories]] entries:
  id                     string            The inventory config's name.
                                            Character-owned inventories
                                            reference this id.
  bag_size               usize             Max number of unequipped items
                                            the bag can hold.
  [[equipment_slots]]    array of tables   Named equip slots. Defaults to
                                            [] if omitted. See "equipment
                                            slots" below."#
        .to_string()
}

fn equipment_slots_section() -> String {
    r#"equipment_slots fields:
  name         string            The slot's name. Must be unique within an
                                  inventory config — a config with duplicate
                                  slot names fails to load.
  item_types   array of strings  Item type strings (matching an item's
                                  item_type, see `mudroom instructions
                                  items`) allowed in this slot. Defaults to
                                  [] if omitted."#
        .to_string()
}

fn character_type_section() -> String {
    r#"Character-owned inventories:
  Every character has an inventory with a `type` naming which
  inventory config shapes it, defaulting to the well-known id
  "inventory". A character whose stored type doesn't match any
  configured id falls back to "inventory" (logged as a warning) rather
  than failing to load."#
        .to_string()
}

fn example_section() -> String {
    r#"Complete annotated example:

muds/basic/inventory.toml:

  [[inventories]]
  id = "inventory"                        # the well-known default
  bag_size = 100

  [[inventories.equipment_slots]]
  name = "left_hand"                      # dual wielding weapons/shields
  item_types = ["weapon", "shield"]

  [[inventories.equipment_slots]]
  name = "right_hand"
  item_types = ["weapon", "shield"]

  [[inventories.equipment_slots]]
  name = "body"
  item_types = ["body_armor"]

  [[inventories.equipment_slots]]
  name = "head"
  item_types = ["head_armor"]

  [[inventories.equipment_slots]]
  name = "feet"
  item_types = ["shoes"]

  [[inventories.equipment_slots]]
  name = "amulet_1"                       # four identically-typed slots,
  item_types = ["amulet"]                 # amulet_1..amulet_4

  [[inventories.equipment_slots]]
  name = "amulet_2"
  item_types = ["amulet"]

  [[inventories.equipment_slots]]
  name = "amulet_3"
  item_types = ["amulet"]

  [[inventories.equipment_slots]]
  name = "amulet_4"
  item_types = ["amulet"]"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_top_level_fields() {
        let text = render();
        assert!(text.contains("id"));
        assert!(text.contains("bag_size"));
        assert!(text.contains("[[equipment_slots]]"));
    }

    #[test]
    fn render_documents_equipment_slot_fields() {
        let text = render();
        assert!(text.contains("item_types"));
        assert!(text.contains("duplicate"));
    }

    #[test]
    fn render_documents_default_inventory_type_fallback() {
        let text = render();
        assert!(text.contains("\"inventory\""));
        assert!(text.contains("falls back"));
    }

    #[test]
    fn render_includes_worked_example() {
        let text = render();
        assert!(text.contains("Complete annotated example"));
        assert!(text.contains("muds/basic/inventory.toml"));
    }
}
