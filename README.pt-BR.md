<p align="center">
  <img width="128" src=".github/assets/logo.png" alt="Logo do schumaz.gg">
</p>
<h1 align="center">schumaz.gg</h1>
<p align="center">
  <strong>Companion App Automatizado para League of Legends.</strong> O schumaz.gg detecta a Seleção de Campeões no
  cliente do LoL e aplica automaticamente runas, feitiços de invocador e conjuntos de itens do OP.GG — além de
  oferecer tier lists Ranqueadas, builds de ARAM &amp; ARAM Desordem (Mayhem) e análise do perfil de invocador. Gratuito
  e de código aberto, para Windows.
</p>

<p align="center">
  <a href="https://github.com/schumaz/schumaz.gg/releases/latest"><img alt="Última release" src="https://img.shields.io/github/v/release/schumaz/schumaz.gg?include_prereleases"></a>
  <a href="https://github.com/schumaz/schumaz.gg/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/schumaz/schumaz.gg/total"></a>
  <a href="LICENSE"><img alt="Licença: GPL-3.0-only" src="https://img.shields.io/badge/licence-GPL--3.0--only-blue"></a>
  <img alt="Plataforma: Windows" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white">
  <img alt="Feito com Tauri v2 & Rust" src="https://img.shields.io/badge/Tauri%20v2-Rust%20%2B%20React-FFC131?logo=tauri&logoColor=white">
</p>

<p align="center">
  <a href="https://github.com/schumaz/schumaz.gg/releases/latest">Download</a>
  ·
  <a href="#funcionalidades">Funcionalidades</a>
  ·
  <a href="#instalação">Instalação</a>
  ·
  <a href="#como-usar">Como usar</a>
  ·
  <a href="#faq">FAQ</a>
  ·
  <a href="https://github.com/schumaz/schumaz.gg/issues/new">Reportar um bug</a>
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>Português (BR)</strong>
</p>

> [!NOTE]
> **Aviso de Não-Endosso da Riot Games:** schumaz.gg isn't endorsed by Riot Games and doesn't reflect the views or
> opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and
> all associated properties are trademarks or registered trademarks of Riot Games, Inc.

> [!WARNING]
> Os únicos downloads oficiais estão na página de [Releases](https://github.com/schumaz/schumaz.gg/releases) deste
> repositório.

<p align="center">
  <img src=".github/assets/champ-select.png" alt="Aba Runas do schumaz.gg: build do campeão carregada com runas primárias e secundárias, fragmentos, feitiços de invocador, progressão de itens em 5 blocos e filtros de elo, rota, região e patch.">
</p>

## Por que existe

Durante a Seleção de Campeões você não deveria precisar ficar dando Alt+Tab para o navegador nem brigar com um overlay
pesado só para copiar runas e itens antes de o tempo acabar. O schumaz.gg conecta-se diretamente ao cliente do League of
Legends, busca os dados consolidados de build no OP.GG e DataDragon e aplica suas runas, feitiços e conjuntos de itens
na loja automaticamente em segundos.

Criei o schumaz.gg por frustração própria. Ao longo dos anos usei vários companions de LoL feitos em Electron, e **todos
eles tinham exatamente os mesmos problemas**: interfaces lentas, queda perceptível de FPS e piora no *1% low* dentro da
partida, demora para aplicar as runas no Champ Select e alto consumo de memória RAM só de ficarem abertos em segundo
plano. Ao construir o schumaz.gg com **Rust** e **Tauri v2** (usando o WebView2 nativo do Windows em vez de embutir
outra instância inteira do Chromium), o aplicativo permanece rápido, leve e fora do caminho enquanto você joga.

## Funcionalidades

- **Automação instantânea no Champ Select**: detecta quando você passa o mouse (*hover*) ou confirma (*lock-in*) um
  campeão no cliente do LoL, com reconciliação final de segurança antes de a partida começar (incluindo trocas de última
  hora no banco do ARAM).
- **Auto-injeção configurável**: ative ou desative de forma independente a aplicação automática de **Runas**, **Feitiços
  de Invocador** (respeitando sua preferência de **Flash no D ou F**) e **Conjuntos de Itens** em 5 blocos na loja.
- **Tier List Ranqueada & Explorador de Builds**: consulte a lista de tiers e builds completas filtradas por **Elo**,
  **Rota**, **Região** (15 servidores + Global) e **Patch**.
- **ARAM & ARAM Desordem (Mayhem)**: suporte dedicado tanto ao **ARAM** clássico quanto ao **ARAM Desordem**, incluindo
  ranking de **Aprimoramentos (Augments)** filtráveis por raridade (*Prata*, *Ouro*, *Prismático*, *Especial*).
- **Análise de Perfil do Invocador**: visualize seu ícone, nível, papel de parede do perfil do cliente, emblemas
  ranqueados, Top 3 maestrias, gráfico de Vitórias/Derrotas e as últimas 10 partidas filtráveis por modo de jogo.
- **Proteção de Novo Patch (48h)**: trava a seleção automática no patch anterior já consolidado durante as primeiras
  48 horas após o lançamento de um novo patch para evitar builds com baixa amostragem (pode ser ignorada manualmente no
  seletor de patch ou desativada nas Configurações).
- **Cache local rápido e baixo consumo**: cache binário comprimido com LZ4 + MessagePack, limpeza automática de arquivos
  antigos, polling adaptativo em idle e opção para desativar efeitos de GPU.
- **Qualidade de vida**: limpeza automática opcional das notificações de missões e objetivos não lidos no cliente do
  LoL, interface bilíngue (**Português BR** e **English**) e atualizações silenciosas em segundo plano que aguardam sua
  partida terminar antes de baixar.

## Instalação

| Requisito | |
|---|---|
| Sistema | Windows 10 ou 11, 64 bits |
| Jogo | Cliente do League of Legends instalado |

Baixe uma das duas opções na página de [Releases](https://github.com/schumaz/schumaz.gg/releases/latest):

- **Instalador (`schumaz.gg_<versão>_x64-setup.exe`)**: instala apenas para o seu usuário (`currentUser`), não exige
  permissões de administrador, possui assistente em Português (BR) e Inglês e atualiza automaticamente em segundo plano.
- **Portátil (`schumaz.gg_<versão>_x64-portable.exe`)**: executável único sem instalação. Avisa quando houver uma nova
  versão no GitHub para você baixar manualmente.

<!-- UNSIGNED NOTICE: remova este bloco quando as releases forem assinadas com certificado EV/OV. -->
> [!IMPORTANT]
> **Os executáveis ainda não possuem assinatura digital (Authenticode)**, então a primeira execução provavelmente
> exibirá o aviso do Windows SmartScreen ("O Windows protegeu o computador"). Clique em **Mais informações** e depois em
> **Executar assim mesmo**. Para conferir a integridade do arquivo, compare o SHA-256 com o arquivo `SHA256SUMS.txt` da
> release (no PowerShell: `Get-FileHash <arquivo>`).
<!-- END UNSIGNED NOTICE -->

## Como usar

1. **Abra o schumaz.gg.** Na primeira abertura, escolha seu idioma (`Português` ou `English`) e sua tecla do Flash (`D`
   ou `F`).
2. **Abra o cliente do League of Legends.** O indicador no topo ficará verde assim que conectar, e a aba **Perfil**
   sincronizará sua conta automaticamente.
3. **Entre na fila (ou navegue por Ranqueada / ARAM antes).** Ao selecionar ou confirmar um campeão no Champion Select,
   o schumaz.gg carrega a build e aplica runas, feitiços e itens conforme as suas preferências.

<p align="center">
  <img src=".github/assets/profile.png" width="49%" alt="Aba Perfil: ícone de invocador, wallpaper de fundo, elos, Top 3 maestrias, gráfico de Vitórias/Derrotas e histórico recente.">
  <img src=".github/assets/ranked.png" width="49%" alt="Aba Ranqueada: Tier List Solo/Duo com filtros de elo, rota, região e patch.">
</p>
<p align="center">
  <img src=".github/assets/aram-mayhem.png" width="49%" alt="Aba ARAM Desordem: ranking de campeões e Aprimoramentos filtráveis por raridade.">
  <img src=".github/assets/settings.png" width="49%" alt="Aba Configurações: posição do Flash, toggles de auto-injeção, proteção de 48h, idioma, GPU e atualizações.">
</p>

## FAQ

**Posso tomar ban? É seguro com o Riot Vanguard?**  
O schumaz.gg se comunica exclusivamente com o cliente do League of Legends através da API local oficial do próprio
cliente (REST e WebSocket). Ele nunca lê nem escreve na memória do processo do jogo, nunca injeta DLLs, nunca altera
arquivos do jogo e não realiza nenhuma automação dentro da partida. Ainda assim, como qualquer ferramenta de terceiros,
o uso é por sua conta e risco.

**De onde vêm as builds e os dados?**  
Estatísticas de builds, tier lists e aprimoramentos do ARAM Desordem vêm do OP.GG, enquanto os ativos e nomes localizados
de campeões, runas, feitiços e itens vêm do DataDragon da Riot (além do CommunityDragon para ativos visuais como brasões
de elo).

**O que ele grava no meu PC e onde?**  
Todas as configurações e caches ficam centralizados em `%LOCALAPPDATA%\schumazgg\`:
- `Config\UserSettings.ini` — suas preferências, posição/tamanho da janela e filtros salvos
- `Cache\` (`ddragon\`, `opgg\`, `profile\`, `assets\`) — caches locais comprimidos com limpeza automática para arquivos antigos
- `WebView\` — dados do runtime WebView2

Os arquivos de instalação do League of Legends nunca são modificados.

**Preciso criar conta?**  
Não. Tudo roda localmente no seu PC, sem login, sem anúncios e sem telemetria.

**Algo quebrou ou uma build parece incorreta.**  
Abra uma [issue](https://github.com/schumaz/schumaz.gg/issues/new) informando o campeão, a fila e a versão do app.

## Compilar do zero (Build from source)

Pré-requisitos: Node.js (LTS), Rust (toolchain MSVC no Windows) e os pré-requisitos padrão do Tauri v2 no Windows
(ferramentas de build C++ do MSVC e WebView2):

```powershell
# Instalar dependências do frontend
npm install

# Executar em modo de desenvolvimento
npm run tauri dev

# Gerar instalador NSIS + executável Portátil em dist-release\
npm run build:desktop -- --no-sign
```

Documentação útil: [ARCHITECTURE.md](ARCHITECTURE.md).

## Licença e Avisos Legais

Licenciado sob **GPL-3.0-only** ([LICENSE](LICENSE)).

schumaz.gg isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially
involved in producing or managing Riot Games properties. Riot Games, League of Legends, and all associated properties
are trademarks or registered trademarks of Riot Games, Inc.

### Forks

Forks são bem-vindos sob a licença GPL-3.0. Se você distribuir um fork, dê a ele um nome próprio, deixe claro que não é
oficial e mantenha os avisos originais de copyright e licença. Os builds oficiais são publicados exclusivamente na página
de [Releases](https://github.com/schumaz/schumaz.gg/releases) deste repositório.
