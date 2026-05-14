use bevy_pmx::PmxAssetLabel;

#[test]
fn morph_labels_match_the_actual_morph_subasset_name() {
    assert_eq!(PmxAssetLabel::Morph(0).to_string(), "Morph/0");
    assert_eq!(PmxAssetLabel::Morph(12).to_string(), "Morph/12");
}
