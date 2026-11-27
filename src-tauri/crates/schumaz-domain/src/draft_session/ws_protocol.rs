/// Parsed LCU WebSocket event emitted for `OnJsonApiEvent_*` subscriptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LcuWsEvent {
    pub topic: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Converts an LCU REST endpoint (e.g. `/lol-champ-select/v1/session`) into its WAMP topic.
pub fn endpoint_to_json_api_topic(endpoint: &str) -> String {
    let trimmed = endpoint.trim_start_matches('/').replace('/', "_");
    format!("OnJsonApiEvent_{trimmed}")
}

/// Builds the WAMP v1 `[5, "<topic>"]` subscribe frame expected by LCU.
pub fn build_subscribe_frame(topic: &str) -> String {
    serde_json::json!([5, topic]).to_string()
}

/// Parses an LCU WAMP `[8, "<topic>", { data, eventType, uri }]` text payload.
pub fn parse_lcu_ws_event(raw: &str) -> Option<LcuWsEvent> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let val: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let arr = val.as_array()?;
    if arr.len() < 3 || arr[0].as_u64() != Some(8) {
        return None;
    }

    let topic = arr[1].as_str()?.to_string();
    let payload = arr[2].as_object()?;
    let event_type = payload.get("eventType")?.as_str()?.to_string();
    let data = payload
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Some(LcuWsEvent {
        topic,
        event_type,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_to_json_api_topic_and_subscribe_frame() {
        assert_eq!(
            endpoint_to_json_api_topic("/lol-champ-select/v1/session"),
            "OnJsonApiEvent_lol-champ-select_v1_session"
        );
        assert_eq!(
            endpoint_to_json_api_topic("/lol-gameflow/v1/gameflow-phase"),
            "OnJsonApiEvent_lol-gameflow_v1_gameflow-phase"
        );
        assert_eq!(
            endpoint_to_json_api_topic("/lol-lobby/v2/lobby"),
            "OnJsonApiEvent_lol-lobby_v2_lobby"
        );
        assert_eq!(
            build_subscribe_frame("OnJsonApiEvent_lol-lobby_v2_lobby"),
            r#"[5,"OnJsonApiEvent_lol-lobby_v2_lobby"]"#
        );
    }

    #[test]
    fn test_parse_lcu_ws_event_valid_and_ignored_frames() {
        let valid = r#"[8,"OnJsonApiEvent_lol-gameflow_v1_gameflow-phase",{"data":"ChampSelect","eventType":"Update","uri":"/lol-gameflow/v1/gameflow-phase"}]"#;
        let parsed = parse_lcu_ws_event(valid).expect("valid opcode 8 event should parse");
        assert_eq!(parsed.topic, "OnJsonApiEvent_lol-gameflow_v1_gameflow-phase");
        assert_eq!(parsed.event_type, "Update");
        assert_eq!(parsed.data, serde_json::json!("ChampSelect"));

        assert!(parse_lcu_ws_event("").is_none());
        assert!(parse_lcu_ws_event(r#"[0,"wamp",1,null]"#).is_none());
        assert!(parse_lcu_ws_event("{not json").is_none());
        assert!(parse_lcu_ws_event(r#"[8,"OnJsonApiEvent_x",{"data":{}}]"#).is_none());
    }
}
