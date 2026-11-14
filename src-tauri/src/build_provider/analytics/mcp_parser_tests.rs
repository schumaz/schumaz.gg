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

#[test]
fn test_parse_opgg_mcp_text_aram_with_runes_id_and_tree_names() {
    let text_aram = "class LolGetChampionAnalysis: data\nclass Data: summary,damage_type,core_items,boots,starter_items,fourth_items,fifth_items,summoner_spells,runes\nclass Summary: average_stats\nclass AverageStats: play,win_rate,pick_rate\nclass CoreItems: ids,play,win,pick_rate\nclass Boots: ids,ids_names,play,win,pick_rate\nclass SummonerSpells: ids,win,play\nclass Runes: id,primary_page_id,primary_page_name,primary_rune_ids,primary_rune_names,secondary_page_id,secondary_page_name,secondary_rune_ids,secondary_rune_names,stat_mod_ids,stat_mod_names,play,win,pick_rate\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(80350,0.51,0.12)),\"AP\",CoreItems([6655,4646,4645],7482,3735,0.13),Boots([3020],[\"Sorcerer's Shoes\"],59262,30401,0.89),CoreItems([2031,3802],47053,24340,0.64),[CoreItems([3089],7106,3655,0.23)],[CoreItems([3135],1951,937,0.19)],SummonerSpells([4,32],21802,42643),Runes(8128,8100,\"Domination\",[8128,8126,8140,8106],[\"Dark Harvest\",\"Cheap Shot\",\"Grisly Mementos\",\"Ultimate Hunter\"],8000,\"Precision\",[8009,8014],[\"Presence of Mind\",\"Coup de Grace\"],[5008,5008,5001],[5008,5008,5001],24553,12358,0.32)))";
    let build = parse_opgg_mcp_text(text_aram, 103, "Ahri")
        .expect("Parsing ARAM MCP payload with Runes id and tree names should succeed");

    assert_eq!(build.primary_style_id, 8100);
    assert_eq!(build.sub_style_id, 8000);
    assert_eq!(
        build.selected_perk_ids,
        vec![8128, 8126, 8140, 8106, 8009, 8014, 5008, 5008, 5001]
    );
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 32);
    assert_eq!(build.active_position, "");
}

#[test]
fn test_parse_opgg_mcp_text_with_tree_names_without_class_header_and_optional_mayhem_runes() {
    let text_no_header = "LolGetChampionAnalysis(Data(Summary(AverageStats(50000,0.53,0.09)),\"AD\",CoreItems([3032,6675,3031],5000,2700,0.2),CoreItems([3006],10000,5300,0.5),CoreItems([1038,2003],8000,4200,0.4),SummonerSpells([4,7],4000,8000),Runes(8008,8000,\"Precision\",[8008,9111,9104,8014],8100,\"Domination\",[8139,8135],[5005,5008,5001],12000,6400,0.45)))";
    let build = parse_opgg_mcp_text(text_no_header, 18, "Tristana")
        .expect("Parsing MCP Runes with id and tree names without class header should succeed");
    assert_eq!(build.primary_style_id, 8000);
    assert_eq!(build.sub_style_id, 8100);
    assert_eq!(
        build.selected_perk_ids,
        vec![8008, 9111, 9104, 8014, 8139, 8135, 5005, 5008, 5001]
    );

    let text_without_runes = "LolGetChampionAnalysis(Data(Summary(AverageStats(50000,0.53,0.09)),\"AD\",CoreItems([3032,6675,3031],5000,2700,0.2),CoreItems([3006],10000,5300,0.5),CoreItems([1038,2003],8000,4200,0.4),SummonerSpells([4,7],4000,8000)))";
    assert!(parse_opgg_mcp_text(text_without_runes, 18, "Tristana").is_err());
    let mayhem_build = parse_opgg_mcp_text_with_options(text_without_runes, 18, "Tristana", false)
        .expect("Parsing ARAM Mayhem MCP payload without Runes should succeed");
    assert!(!mayhem_build.supports_runes);
    assert!(mayhem_build.selected_perk_ids.is_empty());
}
