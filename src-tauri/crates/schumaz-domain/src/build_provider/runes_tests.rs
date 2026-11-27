use super::*;

#[test]
fn test_extract_top_rune_pages_multiple_sorted_by_play() {
    let json = serde_json::json!({
        "runes": [
            {
                "primary_page_id": 8100,
                "secondary_page_id": 8200,
                "primary_rune_ids": [8112, 8126, 8138, 8135],
                "secondary_rune_ids": [8226, 8210],
                "stat_mod_ids": [5008, 5008, 5001],
                "play": 12000,
                "win": 6200
            },
            {
                "primary_page_id": 8000,
                "secondary_page_id": 8100,
                "primary_rune_ids": [8005, 9111, 9104, 8014],
                "secondary_rune_ids": [8139, 8135],
                "stat_mod_ids": [5008, 5008, 5003],
                "play": 45000,
                "win": 23500
            },
            {
                "primary_page_id": 8200,
                "secondary_page_id": 8300,
                "primary_rune_ids": [8214, 8226, 8210, 8236],
                "secondary_rune_ids": [8304, 8345],
                "stat_mod_ids": [5008, 5008, 5001],
                "play": 8000,
                "win": 4100
            },
            {
                "primary_page_id": 8400,
                "secondary_page_id": 8200,
                "primary_rune_ids": [8437, 8401, 8473, 8484],
                "secondary_rune_ids": [8226, 8210],
                "stat_mod_ids": [5008, 5002, 5001],
                "play": 2000,
                "win": 950
            }
        ]
    });

    let pages = extract_top_rune_pages(&json);
    assert_eq!(pages.len(), 3);
    // 1st: 8000 (45,000 games)
    assert_eq!(pages[0].primary_style_id, 8000);
    assert_eq!(pages[0].play, 45000);
    // 2nd: 8100 (12,000 games)
    assert_eq!(pages[1].primary_style_id, 8100);
    assert_eq!(pages[1].play, 12000);
    // 3rd: 8200 (8,000 games)
    assert_eq!(pages[2].primary_style_id, 8200);
    assert_eq!(pages[2].play, 8000);
}

#[test]
fn test_extract_all_mcp_runes_multiple_pages() {
    let text = "LolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001]),Runes(8000,[8005,9111,9104,8014],8100,[8139,8135],[5008,5008,5001])))";
    let pages = extract_all_mcp_runes(text);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].primary_style_id, 8100);
    assert_eq!(pages[1].primary_style_id, 8000);
}

#[test]
fn test_extract_all_mcp_runes_with_id_and_tree_names() {
    let text = "class Runes: id,primary_page_id,primary_page_name,primary_rune_ids,primary_rune_names,secondary_page_id,secondary_page_name,secondary_rune_ids,secondary_rune_names,stat_mod_ids,stat_mod_names,play,win,pick_rate\n\nLolGetChampionAnalysis(Data(Runes(8128,8100,\"Domination\",[8128,8126,8140,8106],[\"Dark Harvest\",\"Cheap Shot\",\"Grisly Mementos\",\"Ultimate Hunter\"],8000,\"Precision\",[8009,8014],[\"Presence of Mind\",\"Coup de Grace\"],[5008,5008,5001],[5008,5008,5001],24553,12358,0.32)))";
    let pages = extract_all_mcp_runes(text);
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].primary_style_id, 8100);
    assert_eq!(pages[0].sub_style_id, 8000);
    assert_eq!(
        pages[0].selected_perk_ids,
        vec![8128, 8126, 8140, 8106, 8009, 8014, 5008, 5008, 5001]
    );
    assert_eq!(pages[0].play, 24553);
    assert_eq!(pages[0].win, 12358);
}
