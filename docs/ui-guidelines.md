# Diretrizes de Interface Desktop Windows (UI/UX)

> Ler este arquivo apenas ao alterar layout, CSS (`src/App.css`) ou componentes visuais (`src/components/`).

## 1. Viewport Lock e Fit-to-Screen
- `html`, `body`, `#root`: `width: 100vw; height: 100vh; overflow: hidden; margin: 0; padding: 0;`
- `.container`: `height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden;`
- Nunca exibir barra de rolagem global; usar `flex: 1; min-height: 0;` no miolo e `overflow-y: auto` apenas dentro das colunas/painéis (`5px` scrollbar com trilho `transparent`).
- Topo fixo (`StatusDock`, `data-tauri-drag-region`) e base fixa (`NavigationDock`).
- `user-select: none;` na raiz; `-webkit-user-drag: none;` em imagens.

## 2. Grid e Responsividade
- Dashboard em 2 colunas (`.dashboard-col-left` para `RuneTree` / aviso ARAM Mayhem + `SpellsBar`, ou `ArenaAugmentsCard` + `ArenaSynergiesCard` no modo Arena; `.dashboard-col-right` dedicada integralmente a `ItemBlocks`).
- Prioridade de habilidades (`SkillOrderPopover`) e confrontos da rota (`LaneMatchupsPopover`, apenas Summoner's Rift) ancorados à direita da barra de filtros no `FiltersHeader`, abrindo matriz `1–18` e colunas *Difíceis* vs. *Favoráveis* em popovers flutuantes sob demanda sem gerar rolagem nas colunas.
- Abaixo de `860px`, colapsa para 1 coluna mantendo fit-to-screen.

## 3. Paleta Dark Amethyst & Animações
- Base `#0d0e12`, cards `#13141a`, hover `#1a1b24` / `#151620`. A barra inferior `.floating-dock` usa a paleta Dark Amethyst estática e opaca centralizada em `src/styles/navigation-dock.css` (tonalidade idêntica em todas as abas, inclusive no Perfil).
- Acento `#7c3aed` (`--accent-primary`), hover `#a78bfa`, sutil `rgba(124, 58, 237, 0.16)`, borda `rgba(124, 58, 237, 0.45)`.
- `.tabs-viewport`: transições direcionais horizontais via `@keyframes` exclusivos do compositor (`opacity` e `transform: translate3d`) em `src/styles/page-transitions.css` (`240ms cubic-bezier(0.22, 1, 0.36, 1)` entrada, `170ms cubic-bezier(0.4, 0, 1, 1)` saída), sincronizadas com a pílula deslizante da `NavigationDock`, dissolve suave do wallpaper do Perfil (`240ms` / `180ms`) e transições hierárquicas de subviews internas (`210ms` / `190ms` / `175ms`). Em modo sem aceleração de GPU (`.gpu-effects-disabled`), degrada para fades curtos apenas de opacidade (`140ms`); em modo Zero-GPU (`.app-minimized`) ou `@media (prefers-reduced-motion: reduce)`, todas as transições e animações são desativadas.

## 4. Tom de Voz & Regra de Ouro
- Copywriting curto, técnico e impessoal (`"Aguardando cliente do LoL"`, `"D"`/`"F"`). Nunca mencionar executáveis (`LeagueClientUx.exe`) ou fontes externas (`OP.GG`) na UI.
- Alterações puramente visuais modificam **apenas CSS e containers**, sem alterar hooks, contratos IPC ou lógica de estado.
