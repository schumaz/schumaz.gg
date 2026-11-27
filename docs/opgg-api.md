# Referência Rápida: OP.GG API

## Endpoints
- **MCP (`POST https://mcp-api.op.gg/mcp`, `method: "tools/call"`):**
  - `lol_get_champion_analysis` — Parâmetros: `{ game_mode, champion, position, desired_output_fields }` (inclui `data.summary.average_stats.{win_rate,pick_rate,play,tier}`)
  - `lol_list_aram_augments` — Parâmetros: `{ champion_id, lang, desired_output_fields: ["data.augments[].{id,desc,tier,performance,popular}"] }` *(fonte primária oficial de augments do ARAM Mayhem em `analytics/aram/mcp_augments.rs`, hidratada 100% com metadados/ícones via `DataDragonCache.arena_augments` / CommunityDragon; o MCP retorna oficialmente `tier >= 3`, então tiers superiores `tier < 3` são complementados em best-effort pelo fallback HTML isolado quando `ARAM_MAYHEM_HTML_FALLBACK_ENABLED = true`)*
- **REST Versions:** `GET https://lol-api-champion.op.gg/api/{region}/champions/ranked/versions` e `GET https://lol-api-champion.op.gg/api/global/champions/arena/versions`
- **REST ARAM Tier List:** `GET https://lol-api-champion.op.gg/api/global/champions/aram` *(fonte primária JSON para ARAM Clássico e lista de campeões do ARAM Mayhem em `analytics/aram.rs`, zero scraping no fluxo normal)*
- **REST Arena Tier List & Build:** `GET https://lol-api-champion.op.gg/api/global/champions/arena` e `GET https://lol-api-champion.op.gg/api/global/champions/arena/{champion_id}`
- **REST Analysis / Positions:** `https://lol-api-champion.op.gg/api/{region}/champions/{mode}/{champion}/{position}` (ver `build_provider/analytics/`)

## Recorte Real (REST JSON)
```json
{
  "data": {
    "summary": {
      "is_rip": false,
      "average_stats": { "win_rate": 0.512, "pick_rate": 0.084, "play": 12000, "tier": 1 },
      "positions": [{ "name": "MID", "stats": { "tier_data": { "tier": 1, "rank": 2 } } }]
    },
    "runes": [{
      "primary_page_id": 8100, "primary_rune_ids": [8112, 8143, 8138, 8106],
      "secondary_page_id": 8200, "secondary_rune_ids": [8210, 8236],
      "stat_mod_ids": [5008, 5008, 5001], "play": 4500, "win": 2350
    }],
    "summoner_spells": [{ "ids": [4, 14], "play": 9000, "win": 4600 }],
    "starter_items": [{ "ids": [1055, 2003], "play": 6000, "win": 3100, "pick_rate": 0.5 }],
    "core_items": [{ "ids": [3142, 6691, 6694], "play": 3200, "win": 1750, "pick_rate": 0.28 }],
    "boots": [{ "ids": [3158], "play": 5000, "win": 2600, "pick_rate": 0.42 }],
    "last_items": [{ "ids": [6692], "play": 1800, "win": 990, "pick_rate": 0.15 }],
    "skill_masteries": [{ "ids": ["Q", "E", "W"], "play": 9500, "win": 5000, "pick_rate": 0.78, "builds": [{ "order": ["Q", "W", "E"], "play": 6000, "win": 3200, "pick_rate": 0.63 }] }],
    "skills": [{ "order": ["Q", "W", "E"], "play": 6000, "win": 3200, "pick_rate": 0.49 }],
    "counters": [{ "champion_id": 950, "play": 511, "win": 239 }]
  }
}
```
> Também suporta wrapper MCP em `/result/content/0/text` (DSL estruturada parseada por `analytics/{mcp_dsl,mcp_parser,mcp_extractors,mcp_stats}.rs` e `analytics/aram/mcp_augments.rs`).
> Normalização: `win_rate`/`pick_rate` ≤ `1.0` são multiplicados por `100` com 1 casa decimal; tiers OP.GG são 1-based (`1 = OP`, `2 = Tier 1`, `3 = Tier 2`, `4 = Tier 3`, `5 = Tier 4`) e normalizados para a escala canônica 0-based `0..=4` em todos os modos (Ranqueada por posição em `summary.positions[]`, ARAM Clássico/Mayhem, Arena e builds no Champ Select; rotas off-meta ou `is_rip == true` retornam `None`).
