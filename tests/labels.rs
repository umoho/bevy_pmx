use bevy_pmx::PmxAssetLabel;

#[test]
fn morph_labels_match_the_actual_morph_subasset_name() {
    assert_eq!(PmxAssetLabel::Morph(0).to_string(), "Morph/0");
    assert_eq!(PmxAssetLabel::Morph(12).to_string(), "Morph/12");
}

#[test]
fn physics_labels_match_the_actual_physics_subasset_name() {
    assert_eq!(PmxAssetLabel::RigidBody(0).to_string(), "RigidBody/0");
    assert_eq!(PmxAssetLabel::Joint(12).to_string(), "Joint/12");
    assert_eq!(PmxAssetLabel::SoftBody(7).to_string(), "SoftBody/7");
}
