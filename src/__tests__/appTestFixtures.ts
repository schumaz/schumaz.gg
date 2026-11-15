import type { ChampionBuild } from "../App";

export const mockAhriBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 51.8,
  pick_rate: 8.4,
  primary_style_id: 8100,
  sub_style_id: 8200,
  selected_perk_ids: [8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 14,
  champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png",
  primary_style_name: "Dominação",
  primary_style_icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png",
  sub_style_name: "Feitiçaria",
  sub_style_icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png",
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
  spell_choices: [
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
    { id: 12, name: "Teleporte", icon_url: "teleport.png" },
  ],
  spells: [
    { id: 4, name: "Flash", icon_url: "flash.png" },
    { id: 14, name: "Incendiar", icon_url: "ignite.png" },
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
      play: 25000,
      win: 13000,
      pick_rate: 65.0,
    },
    {
      name: "Build 2",
      core_item_ids: [3285, 4645, 3089],
      core_item_icon_urls: ["3285.png", "4645.png", "3089.png"],
      final_item_ids: [3285, 4645, 3089, 3157, 3135, 3102],
      final_item_icon_urls: ["3285.png", "4645.png", "3089.png", "3157.png", "3135.png", "3102.png"],
      situational_item_ids: [3165, 4629, 3116],
      situational_item_icon_urls: ["3165.png", "4629.png", "3116.png"],
      play: 8000,
      win: 4200,
      pick_rate: 20.8,
    },
    {
      name: "Build 3",
      core_item_ids: [3285, 3165, 3089],
      core_item_icon_urls: ["3285.png", "3165.png", "3089.png"],
      final_item_ids: [3285, 3165, 3089, 3157, 3135, 4629],
      final_item_icon_urls: ["3285.png", "3165.png", "3089.png", "3157.png", "3135.png", "4629.png"],
      situational_item_ids: [4645, 3102, 3041],
      situational_item_icon_urls: ["4645.png", "3102.png", "3041.png"],
      play: 3000,
      win: 1550,
      pick_rate: 7.8,
    },
  ],
  core_build_options: [
    {
      item_ids: [3285, 3157, 3089],
      item_icon_urls: ["3285.png", "3157.png", "3089.png"],
      play: 25000,
      win: 13000,
      pick_rate: 65.0,
    },
    {
      item_ids: [3285, 4645, 3089],
      item_icon_urls: ["3285.png", "4645.png", "3089.png"],
      play: 8000,
      win: 4200,
      pick_rate: 20.8,
    },
    {
      item_ids: [3285, 3165, 3089],
      item_icon_urls: ["3285.png", "3165.png", "3089.png"],
      play: 3000,
      win: 1550,
      pick_rate: 7.8,
    },
  ],
  final_build_options: [
    {
      item_ids: [3285, 3157, 3089, 3135, 4645, 3102],
      item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"],
      play: 12000,
      win: 6500,
      pick_rate: 55.0,
    },
    {
      item_ids: [3285, 3157, 3089, 4629, 3135, 3102],
      item_icon_urls: ["3285.png", "3157.png", "3089.png", "4629.png", "3135.png", "3102.png"],
      play: 5000,
      win: 2700,
      pick_rate: 22.9,
    },
  ],
  item_blocks: [
    { title: "Itens Iniciais", item_ids: [1056, 2003], item_icon_urls: ["1056.png", "2003.png"] },
    { title: "Primeiros 3 Itens", item_ids: [3285, 3157, 3089], item_icon_urls: ["3285.png", "3157.png", "3089.png"] },
    { title: "3 Botas Mais Feitas", item_ids: [3020, 3111, 3047], item_icon_urls: ["3020.png", "3111.png", "3047.png"] },
    { title: "Build Final", item_ids: [3285, 3157, 3089, 3135, 4645, 3102], item_icon_urls: ["3285.png", "3157.png", "3089.png", "3135.png", "4645.png", "3102.png"] },
    { title: "Itens Situacionais / Úteis", item_ids: [4629, 3165, 3102], item_icon_urls: ["4629.png", "3165.png", "3102.png"] },
  ],
  active_position: "mid",
  active_tier: "emerald_plus",
  active_region: "global",
  active_patch: "16.22",
  available_patches: ["16.22", "16.21"],
  patch_fallback_applied: false,
};

export function defaultAppInvokeMock(cmd: string, args?: Record<string, unknown>) {
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
