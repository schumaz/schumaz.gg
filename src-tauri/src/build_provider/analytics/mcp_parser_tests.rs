use super::*;
use crate::build_provider::*;

#[test]
fn test_parse_opgg_mcp_text_real_ahri_payload() {
    let ahri_mcp_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "class LolGetChampionAnalysis: data\nclass Data: summary,core_items,boots,starter_items,fourth_items,fifth_items,sixth_items,summoner_spells,runes\nclass Summary: average_stats\nclass AverageStats: win_rate,pick_rate\nclass CoreItems: ids\nclass Runes: primary_page_id,primary_rune_ids,secondary_page_id,secondary_rune_ids,stat_mod_ids\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003,2003]),[CoreItems([3089]),CoreItems([3157]),CoreItems([3135])],[CoreItems([3089]),CoreItems([3135]),CoreItems([3157])],[CoreItems([4629]),CoreItems([3135]),CoreItems([4646])],CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))\""
                }
            ]
        }
    });

    let build = parse_opgg_analysis_payload(&ahri_mcp_json, 103, "Ahri")
        .expect("Parsing real Ahri MCP payload should succeed");

    assert_eq!(build.champion_id, 103);
    assert_eq!(build.champion_name, "Ahri");
    // win_rate: 0.51 converted to 51.0%, pick_rate: 0.12 converted to 12.0%
    assert_eq!(build.win_rate, 51.0);
    assert_eq!(build.pick_rate, 12.0);

    // Runes: Domination 8100 + Sorcery 8200
    assert_eq!(build.primary_style_id, 8100);
    assert_eq!(build.sub_style_id, 8200);
    assert_eq!(
        build.selected_perk_ids,
        vec![8112, 8139, 8140, 8106, 8210, 8226, 5005, 5008, 5001]
    );

    // Spells: Flash (4) + Ignite (14)
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 14);

    // Core 3 items: Malignance (3118), Shadowflame (4645), Zhonya's (3157) - NO BOOTS!
    assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
    assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
    for item in &build.item_blocks[1].item_ids {
        assert!(
            !is_boot_item(*item),
            "Boot item {} must not be in Core 3 items",
            item
        );
    }

    assert_eq!(build.item_blocks.len(), 5);
    // Position 3: Boots block: Sorcerer's Shoes (3020)
    assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
    assert!(build.item_blocks[2].item_ids.contains(&3020));

    // Position 4: Build Final
    assert_eq!(build.item_blocks[3].title, "Build Final");
    for &core_item in &build.item_blocks[1].item_ids {
        assert!(build.item_blocks[3].item_ids.contains(&core_item));
    }

    // Position 5: Itens Situacionais / Úteis
    assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
    assert!(!build.item_blocks[4].item_ids.is_empty());
    for &item in &build.item_blocks[4].item_ids {
        assert!(
            !build.item_blocks[1].item_ids.contains(&item),
            "Situational item {} cannot be in Core 3",
            item
        );
        assert!(
            !build.item_blocks[3].item_ids.contains(&item),
            "Situational item {} cannot be in Final Build",
            item
        );
        assert!(
            !is_boot_item(item),
            "Situational item {} cannot be a boot",
            item
        );
    }
}

#[test]
fn test_parse_opgg_mcp_text_with_play_in_average_stats() {
    let text_with_play = "class LolGetChampionAnalysis: data\nclass Data: summary\nclass Summary: average_stats\nclass AverageStats: play,win_rate,pick_rate\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(80342,0.512,0.118)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))";
    let build = parse_opgg_mcp_text(text_with_play, 103, "Ahri")
        .expect("Parsing payload with play field in AverageStats should succeed");
    assert_eq!(build.win_rate, 51.2);
    assert_eq!(build.pick_rate, 11.8);
    assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
    // Build final first 3 items must be identical to clean_core in exact sequential order
    assert_eq!(&build.item_blocks[3].item_ids[..3], &[3118, 4645, 3157]);
}
