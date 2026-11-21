# Referência Rápida: OP.GG API

## Endpoints
- **MCP (`POST https://mcp-api.op.gg/mcp`, `method: "tools/call"`):**
  - `lol_get_champion_analysis` — Parâmetros: `{ game_mode, champion, position, desired_output_fields }`
  - `lol_list_aram_augments` — Parâmetros: `{ champion_id, lang, desired_output_fields: ["data.augments[].{id,desc,tier,performance,popular}"] }` *(fonte primária oficial de augments do ARAM Mayhem em `analytics/aram/mcp_augments.rs`, hidratada 100% com metadados/ícones via `DataDragonCache.arena_augments` / CommunityDragon; o MCP retorna oficialmente `tier >= 3`, então tiers superiores `tier < 3` são complementados em best-effort pelo fallback HTML isolado quando `ARAM_MAYHEM_HTML_FALLBACK_ENABLED = true`)*
- **REST Versions:** `GET https://lol-api-champion.op.gg/api/{region}/champions/ranked/versions`
- **REST ARAM Tier List:** `GET https://lol-api-champion.op.gg/api/global/champions/aram` *(fonte primária JSON para ARAM Clássico e lista de campeões do ARAM Mayhem em `analytics/aram.rs`, zero scraping no fluxo normal)*
- **REST Analysis / Positions:** `https://lol-api-champion.op.gg/api/{region}/champions/{mode}/{champion}/{position}` (ver `build_provider/analytics/`)

## Recorte Real (REST JSON)
```json
{
  "data": {
    "summary": { "average_stats": { "win_rate": 0.512, "pick_rate": 0.084, "play": 12000 } },
    "runes": [{
      "primary_page_id": 8100, "primary_rune_ids": [8112, 8143, 8138, 8106],
      "secondary_page_id": 8200, "secondary_rune_ids": [8210, 8236],
      "stat_mod_ids": [5008, 5008, 5001], "play": 4500, "win": 2350
    }],
    "summoner_spells": [{ "ids": [4, 14], "play": 9000, "win": 4600 }],
    "starter_items": [{ "ids": [1055, 2003], "play": 6000, "win": 3100, "pick_rate": 0.5 }],
    "core_items": [{ "ids": [3142, 6691, 6694], "play": 3200, "win": 1750, "pick_rate": 0.28 }],
    "boots": [{ "ids": [3158], "play": 5000, "win": 2600, "pick_rate": 0.42 }],
    "last_items": [{ "ids": [6692], "play": 1800, "win": 990, "pick_rate": 0.15 }]
  }
}
```
> Também suporta wrapper MCP em `/result/content/0/text` (DSL estruturada parseada por `analytics/{mcp_parser,mcp_extractors,mcp_stats}.rs` e `analytics/aram/mcp_augments.rs`).
> Normalização: `win_rate`/`pick_rate` ≤ `1.0` são multiplicados por `100` com 1 casa decimal.
