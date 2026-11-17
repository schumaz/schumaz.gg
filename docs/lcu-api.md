# Referência Rápida: LCU API (`LeagueClientUx.exe`)

## Endpoints REST usados
- `GET /lol-summoner/v1/current-summoner` → `{ summonerId, gameName, tagLine, displayName, summonerLevel, profileIconId }`
- `GET /lol-gameflow/v1/session` → `{ gameData: { isCustomGame, gameMode, queue: { id, gameMode } } }`
- `GET /lol-champ-select/v1/session` → sessão do draft/lobby
- `PATCH /lol-champ-select/v1/session/my-selection` → `{ spell1Id, spell2Id }` (retentar em `404` durante transição)
- `GET/POST /lol-perks/v1/pages`, `PUT/DELETE /lol-perks/v1/pages/{id}` → página `"%Champ% | schumaz.gg"`
- `PUT /lol-item-sets/v1/item-sets/{summonerId}/sets` → conjuntos de itens na loja
- `GET /lol-objectives/v1/ready` → prontidão do módulo de objetivos (`boolean`)
- `GET /lol-objectives/v1/objectives/{lol,tft,jade}` → categorias e grupos de objetivos com `missions[]` (`id`, `isNew`, `viewed`, `status`, `startTime`, `endTime` — atenção: TFT usa epoch em segundos, LoL em milissegundos)
- `GET /lol-missions/v1/missions` e `GET /lol-missions/v1/series` → listas planas de `PlayerMissionDTO` e séries (filtrar por campos estruturais `status`, `isNew`, `viewed`, `displayType`, `display.locations`, sem depender de nomes)
- `PUT /lol-missions/v1/player` → `{ missionIds: string[], seriesIds: string[] }` (marca missões e séries como vistas/lidas, limpando `isNew` e zerando o contador de notificação; retorna `204`)

## WebSocket & Recortes Reais
Tópico: `OnJsonApiEvent_lol-champ-select_v1_session` (`eventType`: `"Update"` | `"Delete"`).
Tópicos de objetivos/missões: `/lol-objectives/v1/ready`, `/lol-objectives/v1/objectives/{lol,tft,jade}`, `/lol-missions/v1/missions`, `/lol-missions/v1/series`.

```json
{
  "localPlayerCellId": 2,
  "isCustomGame": false,
  "actions": [[
    { "type": "ban", "actorCellId": 2, "championId": 157, "completed": true },
    { "type": "pick", "actorCellId": 2, "championId": 238, "completed": true }
  ]],
  "myTeam": [
    { "cellId": 2, "championId": 238, "championPickIntent": 0, "assignedPosition": "middle" }
  ]
}
```
> Ignorar `type == "ban"`. `assignedPosition`: `"top" | "jungle" | "middle" | "bottom" | "utility"`.
> Filas: Solo/Duo `420`, Flex `440`, Normal `400`/`490`, ARAM `450`, ARAM Mayhem `2400` (`gameMode` contém `"MAYHEM"`), Practice Tool `-1` (`isCustomGame` ou `PRACTICETOOL`).

## Payload de Runas (`POST/PUT /lol-perks/v1/pages`)
```json
{
  "name": "Zed | schumaz.gg",
  "primaryStyleId": 8100,
  "subStyleId": 8200,
  "selectedPerkIds": [8112, 8143, 8138, 8106, 8210, 8236, 5008, 5008, 5001],
  "current": true
}
```
> Sempre reutilizar página com `isDeletable == true` cujo `name` contenha `" | schumaz.gg"`.
