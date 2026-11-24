# Diretrizes de Interface Desktop Windows (UI/UX)

> Ler este arquivo apenas ao alterar layout, CSS (`src/App.css`) ou componentes visuais (`src/components/`).

## 1. Viewport Lock e Fit-to-Screen
- `html`, `body`, `#root`: `width: 100vw; height: 100vh; overflow: hidden; margin: 0; padding: 0;`
- `.container`: `height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden;`
- Nunca exibir barra de rolagem global; usar `flex: 1; min-height: 0;` no miolo e `overflow-y: auto` apenas dentro das colunas/painéis (`5px` scrollbar com trilho `transparent`).
- Topo fixo (`StatusDock`, `data-tauri-drag-region`) e base fixa (`NavigationDock`).
- `user-select: none;` na raiz; `-webkit-user-drag: none;` em imagens.

## 2. Grid e Responsividade
- Dashboard em 2 colunas (`.dashboard-col-left` para `RuneTree` / aviso ARAM Mayhem + `SpellsBar` + `SkillOrderCard`; `.dashboard-col-right` para `BuildSelector` + `ItemBlocks`).
- Abaixo de `840px`, colapsa para 1 coluna mantendo fit-to-screen.

## 3. Paleta Dark Amethyst & Animações
- Base `#0d0e12`, cards `#13141a`, hover `#1a1b24` / `#151620`, vidro `.floating-dock`: `rgba(19, 20, 26, 0.85)` + `backdrop-filter: blur(20px)`.
- Acento `#7c3aed` (`--accent-primary`), hover `#a78bfa`, sutil `rgba(124, 58, 237, 0.16)`, borda `rgba(124, 58, 237, 0.45)`.
- `.tabs-viewport`: crossfade entre abas (`260ms cubic-bezier(0.16, 1, 0.3, 1)` entrada, `220ms ease-out` saída) respeitando `@media (prefers-reduced-motion: reduce)`.

## 4. Tom de Voz & Regra de Ouro
- Copywriting curto, técnico e impessoal (`"Aguardando cliente do LoL"`, `"D"`/`"F"`). Nunca mencionar executáveis (`LeagueClientUx.exe`) ou fontes externas (`OP.GG`) na UI.
- Alterações puramente visuais modificam **apenas CSS e containers**, sem alterar hooks, contratos IPC ou lógica de estado.
