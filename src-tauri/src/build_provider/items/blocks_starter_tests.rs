use super::*;
use crate::build_provider::*;

#[test]
fn test_starter_items_by_position_adc_top_mid_jungle_support() {
    // Standard ADC (Yunara): Doran's Blade (1055, 76k) > Doran's Shield (1054, 12k) + Potion (2003)
    let adc_starters = vec![
        ItemStatEntry {
            ids: vec![1055, 2003],
            play: 76942,
            win: 38123,
            pick_rate: 0.8,
        },
        ItemStatEntry {
            ids: vec![1054, 2003],
            play: 12000,
            win: 6000,
            pick_rate: 0.15,
        },
    ];
    let adc_blocks = build_5_item_blocks_with_depth_lists(
        adc_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "adc",
    );
    assert_eq!(adc_blocks[0].title, "Itens Iniciais");
    assert_eq!(adc_blocks[0].item_ids, vec![1055, 2003]);

    // Standard Top (Tank): Doran's Shield (1054, 40k) > Doran's Blade (1055, 5k) + Potion (2003)
    let top_starters = vec![
        ItemStatEntry {
            ids: vec![1054, 2003],
            play: 40000,
            win: 20000,
            pick_rate: 0.7,
        },
        ItemStatEntry {
            ids: vec![1055, 2003],
            play: 5000,
            win: 2500,
            pick_rate: 0.1,
        },
    ];
    let top_blocks = build_5_item_blocks_with_depth_lists(
        top_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "top",
    );
    assert_eq!(top_blocks[0].item_ids, vec![1054, 2003]);

    // Standard Mid (Mage): Doran's Ring (1056, 30k) > Dark Seal (1082, 5k) + Potion (2003)
    let mid_starters = vec![
        ItemStatEntry {
            ids: vec![1056, 2003],
            play: 30000,
            win: 15000,
            pick_rate: 0.8,
        },
        ItemStatEntry {
            ids: vec![1082, 2003],
            play: 5000,
            win: 2500,
            pick_rate: 0.1,
        },
    ];
    let mid_blocks = build_5_item_blocks_with_depth_lists(
        mid_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "mid",
    );
    assert_eq!(mid_blocks[0].item_ids, vec![1056, 2003]);

    // Jungle: Mosstomper (1102, 50k) > Gustwalker (1103, 20k) > Scorchclaw (1101, 10k) + Potion (2003)
    let jng_starters = vec![
        ItemStatEntry {
            ids: vec![1102, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.6,
        },
        ItemStatEntry {
            ids: vec![1103, 2003],
            play: 20000,
            win: 10000,
            pick_rate: 0.25,
        },
        ItemStatEntry {
            ids: vec![1101, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.15,
        },
    ];
    let jng_blocks = build_5_item_blocks_with_depth_lists(
        jng_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "jungle",
    );
    assert_eq!(jng_blocks[0].item_ids, vec![1102, 2003]);

    // Support: Quest item is pre-granted/bought, must ONLY contain Potion!
    let supp_starters = vec![ItemStatEntry {
        ids: vec![3865, 2003, 2003],
        play: 60000,
        win: 30000,
        pick_rate: 0.95,
    }];
    let supp_blocks = build_5_item_blocks_with_depth_lists(
        supp_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "support",
    );
    assert_eq!(supp_blocks[0].item_ids, vec![2003]);
}

#[test]
fn test_aram_starter_preserves_top_potion_and_allows_no_potion_combos() {
    // 1. Tristana ARAM: #1 [1038, 2003, 2003] (play 1397) vs secondary [1038] (398),
    //    [1042, 3006] (251), [2031, 3051] (121), [2051, 2003] (80).
    //    Must select [1038, 2003] without letting secondary 2031 overwrite potion or forcing 2051.
    let tristana_starters = vec![
        ItemStatEntry {
            ids: vec![2051, 2003],
            play: 80,
            win: 40,
            pick_rate: 0.02,
        },
        ItemStatEntry {
            ids: vec![1038, 2003, 2003],
            play: 1397,
            win: 745,
            pick_rate: 0.415,
        },
        ItemStatEntry {
            ids: vec![1038],
            play: 398,
            win: 222,
            pick_rate: 0.118,
        },
        ItemStatEntry {
            ids: vec![1042, 3006],
            play: 251,
            win: 124,
            pick_rate: 0.075,
        },
        ItemStatEntry {
            ids: vec![2031, 3051],
            play: 121,
            win: 69,
            pick_rate: 0.036,
        },
    ];
    let trist_blocks = build_5_item_blocks_with_depth_lists(
        tristana_starters,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(trist_blocks[0].item_ids, vec![1038, 2003]);

    // 2. Solo B.F. Sword [1038] without potion must NOT force 2003
    let solo_bf = build_5_item_blocks_with_depth_lists(
        vec![
            ItemStatEntry {
                ids: vec![1038],
                play: 900,
                win: 500,
                pick_rate: 0.5,
            },
            ItemStatEntry {
                ids: vec![1038, 2003],
                play: 200,
                win: 100,
                pick_rate: 0.1,
            },
        ],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(solo_bf[0].item_ids, vec![1038]);

    // 3. Two components without potion [1037, 1042] must preserve both without adding 2003
    let two_components = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![1037, 1042],
            play: 1200,
            win: 650,
            pick_rate: 0.4,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(two_components[0].item_ids, vec![1037, 1042]);

    // 4. ARAM exclusive starter [2051] / [3112] without consumables must NOT force 2003
    let horn_only = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2051],
            play: 1500,
            win: 820,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(horn_only[0].item_ids, vec![2051]);

    let orb_only = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![3112],
            play: 1500,
            win: 820,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "",
    );
    assert_eq!(orb_only[0].item_ids, vec![3112]);

    // 5. OP.GG numeric sorting puts consumables first: [2031, 3802] -> [3802, 2031]
    //    and [2003, 2003, 3057, 3070] -> [3057, 3070, 2003]
    let ahri_blocks = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2031, 3802],
            play: 2000,
            win: 1100,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AP",
        "",
    );
    assert_eq!(ahri_blocks[0].item_ids, vec![3802, 2031]);

    let ezreal_blocks = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![2003, 2003, 3057, 3070],
            play: 2000,
            win: 1100,
            pick_rate: 0.5,
        }],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        "AD",
        "",
    );
    assert_eq!(ezreal_blocks[0].item_ids, vec![3057, 3070, 2003]);
}
