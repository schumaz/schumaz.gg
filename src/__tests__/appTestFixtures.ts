import type { AppSettings, ChampionBuild } from "../types";

export const mockAhriBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  active_position: "mid",
  active_tier: "emerald_plus",
  active_region: "global",
  active_patch: "16.22",
  patch_fallback_applied: false,
  win_rate: 51.4,
  pick_rate: 8.4,
  spell1_id: 4,
  spell2_id: 14,
  champion_icon_url: "ahri.png",
  primary_style_id: 8100,
  sub_style_id: 8200,
  selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
  primary_style_name: "Dominação",
  primary_style_icon_url: "domination.png",
  sub_style_name: "Feitiçaria",
  sub_style_icon_url: "sorcery.png",
  perks: [
    { id: 8112, key: "Electrocute", name: "Eletrocutar", icon_url: "electrocute.png" },
    { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheapshot.png" },
    { id: 8138, key: "EyeballCollection", name: "Globos Oculares", icon_url: "eyeball.png" },
    { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "treasure.png" },
    { id: 8226, key: "ManaflowBand", name: "Faixa de Fluxo", icon_url: "manaflow.png" },
    { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "transcendence.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
    { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
    { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
  ],
  rune_pages: [
    {
      primary_style_id: 8100,
      sub_style_id: 8200,
      selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
      play: 45000,
      win: 23500,
      pick_rate: 62.5,
      primary_style_name: "Dominação",
      primary_style_icon_url: "domination.png",
      sub_style_name: "Feitiçaria",
      sub_style_icon_url: "sorcery.png",
      perks: [
        { id: 8112, key: "Electrocute", name: "Eletrocutar", icon_url: "electrocute.png" },
        { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheapshot.png" },
        { id: 8138, key: "EyeballCollection", name: "Globos Oculares", icon_url: "eyeball.png" },
        { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "treasure.png" },
        { id: 8226, key: "ManaflowBand", name: "Faixa de Fluxo", icon_url: "manaflow.png" },
        { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "transcendence.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
      ],
    },
    {
      primary_style_id: 8300,
      sub_style_id: 8100,
      selected_perk_ids: [8351, 8304, 8345, 8352, 8126, 8135, 5008, 5008, 5001],
      play: 12000,
      win: 6200,
      pick_rate: 16.7,
      primary_style_name: "Inspiração",
      primary_style_icon_url: "inspiration.png",
      sub_style_name: "Dominação",
      sub_style_icon_url: "domination.png",
      perks: [
        { id: 8351, key: "GlacialAugment", name: "Aprimoramento Glacial", icon_url: "glacial.png" },
        { id: 8304, key: "MagicalFootwear", name: "Calçados Mágicos", icon_url: "footwear.png" },
        { id: 8345, key: "BiscuitDelivery", name: "Entrega de Biscoitos", icon_url: "biscuits.png" },
        { id: 8352, key: "TimeWarpTonic", name: "Tônico de Distorção", icon_url: "timewarp.png" },
        { id: 8126, key: "CheapShot", name: "Golpe Desleal", icon_url: "cheapshot.png" },
        { id: 8135, key: "TreasureHunter", name: "Caçador de Tesouros", icon_url: "treasure.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
      ],
    },
    {
      primary_style_id: 8200,
      sub_style_id: 8300,
      selected_perk_ids: [8229, 8226, 8210, 8237, 8304, 8345, 5008, 5008, 5001],
      play: 8000,
      win: 4100,
      pick_rate: 11.1,
      primary_style_name: "Feitiçaria",
      primary_style_icon_url: "sorcery.png",
      sub_style_name: "Inspiração",
      sub_style_icon_url: "inspiration.png",
      perks: [
        { id: 8229, key: "ArcaneComet", name: "Cometa Arcano", icon_url: "comet.png" },
        { id: 8226, key: "ManaflowBand", name: "Faixa de Fluxo", icon_url: "manaflow.png" },
        { id: 8210, key: "Transcendence", name: "Transcendência", icon_url: "transcendence.png" },
        { id: 8237, key: "Scorch", name: "Chamuscar", icon_url: "scorch.png" },
        { id: 8304, key: "MagicalFootwear", name: "Calçados Mágicos", icon_url: "footwear.png" },
        { id: 8345, key: "BiscuitDelivery", name: "Entrega de Biscoitos", icon_url: "biscuits.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5008, key: "AdaptiveForce", name: "Força Adaptativa", icon_url: "adaptive.png" },
        { id: 5001, key: "HealthScaling", name: "Vida Escalonável", icon_url: "health.png" },
      ],
    },
  ],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
  ],
  spell_choices: [
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
    { id: 12, name: "Teleporte", icon_url: "teleport.png" },
    { id: 3, name: "Exaustão", icon_url: "exhaust.png" },
    { id: 7, name: "Curar", icon_url: "heal.png" },
    { id: 21, name: "Barreira", icon_url: "barrier.png" },
    { id: 6, name: "Fantasma", icon_url: "ghost.png" },
    { id: 1, name: "Purificar", icon_url: "cleanse.png" },
  ],
  build_options: [
    {
      name: "Build 1 (Mais Feita)",
      core_item_ids: [3285, 3157, 3089],
      core_item_icon_urls: ["3285.png", "3157.png", "3089.png"],
      final_item_ids: [3285, 3157, 3089, 3135, 4645, 3102],
      final_item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"],
      situational_item_ids: [4629, 3165, 3102],
      situational_item_icon_urls: ["4629.png", "3165.png", "3102.png"],
      play: 12500,
      win: 6800,
      pick_rate: 57.3,
    },
    {
      name: "Build 2",
      core_item_ids: [6653, 3157, 3089],
      core_item_icon_urls: ["6653.png", "3157.png", "3089.png"],
      final_item_ids: [6653, 3157, 3089, 3135, 4645, 3102],
      final_item_icon_urls: ["6653.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"],
      situational_item_ids: [4629, 3165, 3116],
      situational_item_icon_urls: ["4629.png", "3165.png", "3116.png"],
      play: 6800,
      win: 3600,
      pick_rate: 31.2,
    },
  ],
  item_blocks: [
    { title: "1. Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["1056.png", "2003.png"] },
    { title: "2. Primeiros 3 Itens", item_ids: [3285, 3157, 3089], item_icon_urls: ["3285.png", "3157.png", "3089.png"] },
    { title: "3. 3 Botas Mais Feitas", item_ids: [3020, 3158, 3111], item_icon_urls: ["3020.png", "3158.png", "3111.png"] },
    { title: "4. Build Final", item_ids: [3285, 3157, 3089, 3135, 4645, 3102], item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"] },
    { title: "5. Itens Situacionais / Úteis", item_ids: [4629, 3165, 3102], item_icon_urls: ["4629.png", "3165.png", "3102.png"] },
  ],
};

export function makeDefaultMockAppSettings(): AppSettings {
  return {
    general: {
      language: "pt_BR",
      languageOnboardingCompleted: true,
      flashOnboardingCompleted: true,
    },
    automation: {
      autoInjectRunes: true,
      autoInjectSpells: true,
      autoInjectItems: true,
      flashKey: "D",
      patch48hProtection: true,
      autoMarkDailyMissionsRead: false,
      language: "pt_BR",
    },
    profileAppearance: {
      profileBlurPercent: 10,
      wallpaperVisible: true,
    },
    graphics: {
      hardwareAcceleration: true,
    },
    window: {
      windowWidth: 1280,
      windowHeight: 800,
      windowX: null,
      windowY: null,
      windowMaximized: false,
    },
    navigation: {
      savedAramMode: null,
      profileMatchModeFilter: "all",
      aramMayhemRarityFilter: "all",
      aramMayhemLastChampion: null,
      rankedFilterRegion: "global",
      rankedFilterTier: "emerald_plus",
      rankedFilterLane: "all",
      rankedFilterPatch: null,
      rankedChampionTierFilter: "all",
      lastSummonerId: null,
    },
  };
}

export let activeMockAppSettings: AppSettings = makeDefaultMockAppSettings();

export function resetMockAppSettings(overrides?: Partial<AppSettings>): void {
  activeMockAppSettings = makeDefaultMockAppSettings();
  if (overrides) {
    if (overrides.general) {
      activeMockAppSettings.general = { ...activeMockAppSettings.general, ...overrides.general };
    }
    if (overrides.automation) {
      activeMockAppSettings.automation = { ...activeMockAppSettings.automation, ...overrides.automation };
    }
    if (overrides.profileAppearance) {
      activeMockAppSettings.profileAppearance = { ...activeMockAppSettings.profileAppearance, ...overrides.profileAppearance };
    }
    if (overrides.graphics) {
      activeMockAppSettings.graphics = { ...activeMockAppSettings.graphics, ...overrides.graphics };
    }
    if (overrides.window) {
      activeMockAppSettings.window = { ...activeMockAppSettings.window, ...overrides.window };
    }
    if (overrides.navigation) {
      activeMockAppSettings.navigation = { ...activeMockAppSettings.navigation, ...overrides.navigation };
    }
  }
}

export function defaultAppInvokeMock(cmd: string, args?: Record<string, unknown>) {
  if (cmd === "get_app_settings") {
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "sync_user_preferences") {
    if (args?.preferences) {
      activeMockAppSettings.automation = args.preferences as any;
    }
    return Promise.resolve(null);
  }
  if (cmd === "set_locale") {
    if (args?.locale) {
      activeMockAppSettings.general.language = args.locale as any;
      activeMockAppSettings.automation.language = args.locale as any;
    }
    return Promise.resolve(null);
  }
  if (cmd === "complete_language_onboarding") {
    if (args?.locale) {
      activeMockAppSettings.general.language = args.locale as any;
      activeMockAppSettings.general.languageOnboardingCompleted = true;
      activeMockAppSettings.automation.language = args.locale as any;
    }
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "complete_flash_onboarding") {
    if (args?.flashKey) {
      activeMockAppSettings.general.flashOnboardingCompleted = true;
      activeMockAppSettings.automation.flashKey = args.flashKey as any;
    }
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "update_profile_appearance") {
    activeMockAppSettings.profileAppearance.profileBlurPercent = Number(args?.blurPercent ?? 10);
    activeMockAppSettings.profileAppearance.wallpaperVisible = Boolean(args?.wallpaperVisible ?? true);
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "update_navigation_state") {
    activeMockAppSettings.navigation = {
      ...activeMockAppSettings.navigation,
      ...(args?.patch as any),
    };
    return Promise.resolve(activeMockAppSettings);
  }
  if (cmd === "get_hardware_acceleration_status") {
    return Promise.resolve({
      enabled: activeMockAppSettings.graphics.hardwareAcceleration,
      active_at_startup: true,
    });
  }
  if (
    cmd === "get_summoner_status" ||
    cmd === "get_player_profile" ||
    cmd === "get_cached_player_profile"
  ) {
    return Promise.resolve(null);
  }
  if (cmd === "get_recommended_build_for_champion") {
    if (args?.queueId === 2400) {
      return Promise.resolve({
        ...mockAhriBuild,
        active_position: "",
        supports_runes: false,
        rune_pages: [],
      });
    }
    if (args?.queueId === 450) {
      return Promise.resolve({
        ...mockAhriBuild,
        active_position: "",
        supports_runes: true,
      });
    }
    return Promise.resolve(mockAhriBuild);
  }
  if (cmd === "update_build_filters") {
    return Promise.resolve({
      ...mockAhriBuild,
      active_position: (args?.position as string) || mockAhriBuild.active_position,
      active_tier: (args?.tier as string) || mockAhriBuild.active_tier,
      active_region: (args?.region as string) || mockAhriBuild.active_region,
      active_patch: (args?.patch as string) || mockAhriBuild.active_patch,
    });
  }
  return Promise.resolve({});
}
