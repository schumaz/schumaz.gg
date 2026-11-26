use crate::lcu_connector::client::LcuConnection;

impl LcuConnection {
    /// Fetches the current rune page via GET /lol-perks/v1/currentpage.
    pub async fn get_current_rune_page(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(
            crate::lcu_injector::PERKS_CURRENT_PAGE_ENDPOINT,
            "current rune page",
        )
        .await
    }

    /// Fetches all rune pages via GET /lol-perks/v1/pages.
    pub async fn get_all_rune_pages(&self) -> Result<serde_json::Value, String> {
        self.get_json_required(crate::lcu_injector::PERKS_PAGES_ENDPOINT, "rune pages")
            .await
    }

    /// Updates an existing rune page in-place via PUT /lol-perks/v1/pages/{id}.
    pub async fn put_rune_page(
        &self,
        page_id: i64,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        self.put_json_required(
            &endpoint,
            payload,
            &format!("Error updating rune page {}", page_id),
            "LCU returned an empty response when updating rune page",
        )
        .await
    }

    /// Deletes a rune page by ID via DELETE /lol-perks/v1/pages/{id}.
    pub async fn delete_rune_page(&self, page_id: i64) -> Result<(), String> {
        let endpoint = format!("{}/{}", crate::lcu_injector::PERKS_PAGES_ENDPOINT, page_id);
        self.delete_ignoring_eof(&endpoint, &format!("Error deleting rune page {}", page_id))
            .await
    }

    /// Creates a new rune page via POST /lol-perks/v1/pages.
    pub async fn create_rune_page(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.post_json_required(
            crate::lcu_injector::PERKS_PAGES_ENDPOINT,
            payload,
            "Error creating rune page",
            "LCU returned an empty response when creating rune page",
        )
        .await
    }

    /// Updates summoner spells via PATCH /lol-champ-select/v1/session/my-selection.
    /// Retries up to 5 times with 200ms backoff if champ select session is transitioning.
    pub async fn patch_summoner_spells(&self, payload: &serde_json::Value) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 1..=5 {
            match self
                .raw_client()
                .patch::<&serde_json::Value, Option<serde_json::Value>>(
                    crate::lcu_injector::CHAMP_SELECT_MY_SELECTION_ENDPOINT,
                    payload,
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(err) => {
                    let err_str = format!("{:?}", err);
                    if Self::is_eof_error(&err_str) {
                        return Ok(());
                    }
                    last_err = format!("Error patching summoner spells: {}", err_str);
                    if attempt < 5 && (err_str.contains("404") || err_str.contains("NotFound")) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        continue;
                    }
                    break;
                }
            }
        }
        Err(last_err)
    }

    /// Fetches custom item sets via GET /lol-item-sets/v1/item-sets/{summoner_id}/sets.
    pub async fn get_item_sets(&self, summoner_id: u64) -> Result<serde_json::Value, String> {
        let endpoint = build_item_sets_endpoint(summoner_id);
        let val = self.get_json_required(&endpoint, "item sets").await?;
        if super::is_lcu_error_payload(&val) {
            let status = val
                .get("httpStatus")
                .and_then(|s| s.as_i64())
                .unwrap_or(0);
            let code = val
                .get("errorCode")
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN");
            return Err(format!(
                "LCU returned an error payload for item sets (status={status}, code={code})"
            ));
        }
        Ok(val)
    }

    /// Updates custom item sets via PUT /lol-item-sets/v1/item-sets/{summoner_id}/sets.
    pub async fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let endpoint = build_item_sets_endpoint(summoner_id);
        self.put_json_ignoring_eof(&endpoint, payload, "Error putting item sets")
            .await
    }
}

/// Builds the LCU item sets endpoint for a given `summoner_id`.
pub fn build_item_sets_endpoint(summoner_id: u64) -> String {
    format!("/lol-item-sets/v1/item-sets/{}/sets", summoner_id)
}
