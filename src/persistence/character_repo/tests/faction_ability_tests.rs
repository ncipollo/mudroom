use super::super::*;
use super::support::{setup, test_location};
use crate::persistence::database::Database;

#[tokio::test]
async fn find_by_id_loads_factions_and_faction_relations() {
    use crate::game::component::Faction;
    use crate::game::component::faction_relations::FactionRelation;
    use crate::persistence::{faction_relations_repo, faction_repo};

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    faction_repo::upsert(
        db.pool(),
        &Faction {
            id: "enemy".to_string(),
            name: "Enemy".to_string(),
            description: "Hostile creatures.".to_string(),
        },
    )
    .await
    .unwrap();
    faction_repo::set_character_factions(db.pool(), id, &["enemy".to_string()])
        .await
        .unwrap();

    use crate::game::component::FactionRelations;
    faction_relations_repo::set_character_relations(
        db.pool(),
        id,
        &FactionRelations::default_for_enemy(),
    )
    .await
    .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert!(found.factions.contains("enemy"));
    assert_eq!(
        found.faction_relations.player_relation(),
        &FactionRelation::Hostile
    );
}

#[tokio::test]
async fn find_by_id_uses_character_defaults_when_no_faction_data() {
    use crate::game::component::faction_relations::FactionRelation;

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert!(found.factions.contains("enemy"));
    assert_eq!(
        found.faction_relations.player_relation(),
        &FactionRelation::Hostile
    );
}

#[tokio::test]
async fn find_by_id_loads_innate_abilities() {
    use crate::game::component::{Ability, AbilityRole};
    use crate::game::engagement::EngagementType;
    use crate::persistence::ability_repo;

    let db = Database::connect_in_memory().await.unwrap();
    setup(&db).await;

    let character = Character::new(0, CharacterType::Enemy, test_location());
    let id = insert(db.pool(), &character).await.unwrap();

    let ability = Ability {
        id: "strike".to_string(),
        name: "Strike".to_string(),
        description: Description::default(),
        effects: vec![],
        costs: vec![],
        modifiers: vec![],
        engagement_types: vec![EngagementType::Battle],
        role: AbilityRole::Attack,
        targets: vec![],
        action_text: None,
    };
    ability_repo::upsert(db.pool(), &ability).await.unwrap();
    ability_repo::set_character_abilities(db.pool(), id, &["strike"])
        .await
        .unwrap();

    let found = find_by_id(db.pool(), id).await.unwrap().unwrap();
    assert_eq!(found.innate_abilities.len(), 1);
    assert_eq!(found.innate_abilities[0].id, "strike");
}
