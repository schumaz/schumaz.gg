# Referência Rápida: OP.GG API

## Endpoints
- **MCP (`POST https://mcp-api.op.gg/mcp`, `method: "tools/call"`):**
  - `lol_get_champion_analysis` — Parâmetros: `{ game_mode, champion, position, desired_output_fields }`
  - `lol_list_aram_augments` — Parâmetros: `{ champion_id, lang }` *(rankings de augments do ARAM Mayhem; fonte primária alvo na Frente 4 do `P12.2`, hidratada com metadados/ícones via `DataDragonCache.arena_augments` / CommunityDragon)*
- **REST Versions:** `GET https://lol-api-champion.op.gg/api/{region}/champions/ranked/versions`
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
> Também suporta wrapper MCP em `/result/content/0/text` (markdown/texto estruturado parseado por `analytics/{mcp_parser,mcp_extractors,mcp_stats}.rs`).
> Normalização: `win_rate`/`pick_rate` ≤ `1.0` são multiplicados por `100` com 1 casa decimal.
