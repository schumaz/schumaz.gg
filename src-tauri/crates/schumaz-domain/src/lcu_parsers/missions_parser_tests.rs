use super::*;

#[test]
fn test_normalize_epoch_millis_handles_seconds_and_millis() {
    assert_eq!(normalize_epoch_millis(-1), 0);
    assert_eq!(normalize_epoch_millis(0), 0);
    // 10-digit epoch seconds (used by TFT missions/series)
    assert_eq!(normalize_epoch_millis(1_794_398_400), 1_794_398_400_000);
    // 13-digit epoch milliseconds (used by LoL missions)
    assert_eq!(normalize_epoch_millis(1_794_679_200_000), 1_794_679_200_000);
}

#[test]
fn test_extract_unread_mission_ids_from_objectives_is_name_agnostic_and_clears_all_tabs() {
    let now_ms = 1_794_700_000_000i64;
    let now_secs = now_ms / 1000;
    let objectives = serde_json::json!([
        {
            "gameType": "tft",
            "objectivesCategories": [
                {
                    "id": "cat-renamed-next-season",
                    "startDate": now_secs - 100,
                    "endDate": now_secs + 1000,
                    "objectives": [
                        {
                            "id": "Completely_New_Group_Name_2027",
                            "isEnabled": true,
                            "localizedTag": "Whatever",
                            "localizedTitle": ["Future Mode"],
                            "tag": [],
                            "startDate": now_secs - 100,
                            "endDate": now_secs + 1000,
                            "missions": [
                                {
                                    "id": "tft-weekly-new-1",
                                    "internalName": "Future_Set_Mission_A",
                                    "isNew": true,
                                    "viewed": false,
                                    "status": "PENDING",
                                    "startTime": now_secs - 50,
                                    "endTime": 9223372036854776000u64
                                },
                                {
                                    "id": "lol-weekly-not-viewed",
                                    "internalName": "Arbitrary_Objective_B",
                                    "isNew": false,
                                    "viewed": false,
                                    "status": "COMPLETED",
                                    "startTime": now_ms - 5000,
                                    "endTime": now_ms + 50000
                                },
                                {
                                    "id": "already-read",
                                    "isNew": false,
                                    "viewed": true,
                                    "status": "PENDING",
                                    "startTime": now_ms - 5000,
                                    "endTime": now_ms + 50000
                                },
                                {
                                    "id": "dummy-mission-ignored",
                                    "isNew": true,
                                    "viewed": false,
                                    "status": "DUMMY",
                                    "startTime": now_ms - 5000,
                                    "endTime": now_ms + 50000
                                },
                                {
                                    "id": "expired-mission-ignored",
                                    "isNew": true,
                                    "viewed": false,
                                    "status": "PENDING",
                                    "startTime": now_secs - 500,
                                    "endTime": now_secs - 1
                                },
                                {
                                    "id": "tft-weekly-new-1",
                                    "isNew": true,
                                    "viewed": false,
                                    "status": "PENDING",
                                    "startTime": now_secs - 50,
                                    "endTime": now_secs + 500
                                }
                            ]
                        },
                        {
                            "id": "disabled-group",
                            "isEnabled": false,
                            "missions": [
                                {
                                    "id": "disabled-group-mission",
                                    "isNew": true,
                                    "viewed": false,
                                    "status": "PENDING"
                                }
                            ]
                        }
                    ]
                }
            ]
        }
    ]);

    let ids = extract_unread_mission_ids_from_objectives(&objectives, now_ms);
    assert_eq!(ids, vec!["tft-weekly-new-1", "lol-weekly-not-viewed"]);
}

#[test]
fn test_extract_unread_mission_ids_from_flat_list_uses_structure_not_names() {
    let now_ms = 1_794_700_000_000i64;
    let now_secs = now_ms / 1000;
    let missions = serde_json::json!([
        {
            "id": "visible-tracker-unread",
            "internalName": "Totally_Renamed_Event_Mission",
            "displayType": "ALWAYS",
            "display": { "locations": ["TFT_WEEKLY", "LCU_TRACKER"] },
            "isNew": false,
            "viewed": false,
            "status": "PENDING",
            "startTime": now_secs - 60,
            "endTime": now_secs + 600
        },
        {
            "id": "hidden-but-flagged-is-new",
            "internalName": "Some_Future_Hidden_Mission",
            "displayType": "",
            "display": { "locations": [] },
            "isNew": true,
            "viewed": false,
            "status": "COMPLETED",
            "startTime": now_ms - 1000,
            "endTime": now_ms + 10000
        },
        {
            "id": "internal-boon-without-locations-or-is-new",
            "internalName": "Internal_Game_Mechanic",
            "displayType": "",
            "display": { "locations": [] },
            "isNew": false,
            "viewed": false,
            "status": "PENDING",
            "startTime": 0,
            "endTime": 0
        },
        {
            "id": "dummy-npe-step",
            "displayType": "ALWAYS",
            "display": { "locations": ["NPE_REWARDS_DAILY_PLAY"] },
            "objectives": [{ "status": "DUMMY" }],
            "isNew": false,
            "viewed": false,
            "status": "PENDING",
            "startTime": now_ms - 1000,
            "endTime": now_ms + 10000
        }
    ]);

    let ids = extract_unread_mission_ids_from_missions(&missions, now_ms);
    assert_eq!(
        ids,
        vec!["visible-tracker-unread", "hidden-but-flagged-is-new"]
    );
}

#[test]
fn test_extract_unread_series_ids_filters_and_normalizes_timestamps() {
    let now_ms = 1_794_700_000_000i64;
    let now_secs = now_ms / 1000;
    let series = serde_json::json!([
        {
            "id": "series-tracker-unread",
            "status": "ACTIVE",
            "displayType": "TRACKER",
            "viewed": false,
            "startDate": now_secs - 100,
            "endDate": 0
        },
        {
            "id": "series-already-viewed",
            "status": "ACTIVE",
            "displayType": "TRACKER",
            "viewed": true,
            "startDate": now_secs - 100,
            "endDate": 0
        },
        {
            "id": "series-internal-ignored",
            "status": "ACTIVE",
            "displayType": "INTERNAL",
            "viewed": false,
            "startDate": 0,
            "endDate": 0
        },
        {
            "id": "series-expired-ignored",
            "status": "ACTIVE",
            "displayType": "TRACKER",
            "viewed": false,
            "startDate": now_secs - 200,
            "endDate": now_secs - 10
        }
    ]);

    let ids = extract_unread_series_ids(&series, now_ms);
    assert_eq!(ids, vec!["series-tracker-unread"]);
}
