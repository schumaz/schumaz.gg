use super::*;
use crate::build_provider::ItemStatEntry;

#[test]
fn test_extract_top_core_and_final_build_options() {
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

    let fourth = vec![
        ItemStatEntry {
            ids: vec![3157],
            play: 10000,
            win: 5500,
            pick_rate: 40.0,
        },
        ItemStatEntry {
            ids: vec![3135],
            play: 3000,
            win: 1600,
            pick_rate: 15.0,
        },
    ];
    let fifth = vec![ItemStatEntry {
        ids: vec![3135],
        play: 8000,
        win: 4200,
        pick_rate: 35.0,
    }];
    let sixth = vec![
        ItemStatEntry {
            ids: vec![4629],
            play: 6000,
            win: 3100,
            pick_rate: 30.0,
        },
        ItemStatEntry {
            ids: vec![3102],
            play: 2000,
            win: 1000,
            pick_rate: 10.0,
        },
    ];

    let final_opts = build_3_final_build_options(
        &core_opts[0].item_ids,
        &fourth,
        &fifth,
        &sixth,
        &candidates,
        "AP",
    );
    assert_eq!(final_opts.len(), 3);
    for opt in &final_opts {
        assert_eq!(opt.item_ids.len(), 6);
        assert_eq!(&opt.item_ids[0..3], &[3118, 4645, 3089]);
    }
}
