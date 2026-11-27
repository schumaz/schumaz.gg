use super::*;
use crate::build_provider::ItemStatEntry;

#[test]
fn test_extract_top_core_options() {
    let core_entries = vec![
        ItemStatEntry {
            ids: vec![3118, 4645, 3089],
            play: 15000,
            win: 8000,
            pick_rate: 60.0,
        },
        ItemStatEntry {
            ids: vec![3118, 3157, 3089],
            play: 5000,
            win: 2600,
            pick_rate: 20.0,
        },
        ItemStatEntry {
            ids: vec![3118, 4645, 3135],
            play: 2000,
            win: 1050,
            pick_rate: 8.0,
        },
        ItemStatEntry {
            ids: vec![3118, 3165, 3089],
            play: 500,
            win: 250,
            pick_rate: 2.0,
        },
    ];
    let candidates = vec![3118, 4645, 3089, 3157, 3135, 3165, 4629, 3102];
    let core_opts = extract_top_core_build_options(&core_entries, &candidates, "AP");
    assert_eq!(core_opts.len(), 3);
    assert_eq!(core_opts[0].item_ids, vec![3118, 4645, 3089]);
    assert_eq!(core_opts[0].play, 15000);
    assert_eq!(core_opts[1].item_ids, vec![3118, 3157, 3089]);
    assert_eq!(core_opts[1].play, 5000);
    assert_eq!(core_opts[2].item_ids, vec![3118, 4645, 3135]);
    assert_eq!(core_opts[2].play, 2000);
}
