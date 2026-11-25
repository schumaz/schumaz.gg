# Referência Rápida: LCU API (`LeagueClientUx.exe`)

## Endpoints REST usados
- `GET /lol-summoner/v1/current-summoner` → `{ summonerId, gameName, tagLine, displayName, summonerLevel, profileIconId }`
- `GET /lol-gameflow/v1/session` → `{ gameData: { isCustomGame, gameMode, queue: { id, gameMode }, teamOne, teamTwo, playerChampionSelections } }`
- `GET /lol-lobby/v2/lobby` → `{ gameConfig: { queueId, gameMode }, localMember: { playerSlots: [{ championId, positionPreference, spell1, spell2, perks, skinId }] } }`
- `GET / PUT /lol-lobby/v1/lobby/members/localMember/player-slots` → lê/atualiza os slots de pré-seleção no lobby do **Swiftplay (`480`)** / **Quickplay (`490`)** (`perks` JSON string + `spell1`/`spell2`, preservando `skinId`/`positionPreference`)
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
Tópico de pré-seleção em lobby: `OnJsonApiEvent_lol-lobby_v2_lobby` (`eventType`: `"Create"` | `"Update"` | `"Delete"`).
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
> Filas: Solo/Duo `420`, Flex `440`, Normal `400`, Swiftplay `480` (pré-seleção em `/lol-lobby/v2/lobby` com partição de cache própria `480`), Quickplay `490`, Clash `700` (redireciona para builds Ranqueada `420` sem base própria), ARAM `450`, ARAM Mayhem `2400` (`gameMode` contém `"MAYHEM"`), Arena `1700`/`1710`/`1740`/`1750` (`CHERRY`/`ARENA`/`TGR` ou Mapa `30`), Practice Tool `-1` (`isCustomGame` ou `PRACTICETOOL`).

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
