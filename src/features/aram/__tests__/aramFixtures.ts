import type {
  AramAugmentEntry_Serialize,
  AramChampionRank_Serialize,
} from "../../../types";

export const mockAramChampions: AramChampionRank_Serialize[] = [
  {
    id: 50,
    key: "swain",
    name: "Swain",
    icon_url: "https://example.com/swain.png",
    tier: 0,
    rank: 1,
    win_rate: 63.3,
    pick_rate: 9.1,
  },
  {
    id: 21,
    key: "missfortune",
    name: "Miss Fortune",
    icon_url: "https://example.com/mf.png",
    tier: 1,
    rank: 2,
    win_rate: 52.8,
    pick_rate: 13.7,
  },
];

export const mockMayhemChampions: AramChampionRank_Serialize[] = [
  {
    id: 4,
    key: "twistedfate",
    name: "Twisted Fate",
    icon_url: "https://example.com/tf.png",
    tier: 0,
    rank: 1,
    win_rate: null,
    pick_rate: null,
  },
  {
    id: 103,
    key: "ahri",
    name: "Ahri",
    icon_url: "https://example.com/ahri.png",
    tier: 1,
    rank: 14,
    win_rate: null,
    pick_rate: null,
  },
];

export const mockAhriAugments: AramAugmentEntry_Serialize[] = [
  {
    id: 1074,
    name: "Raio Encolhedor",
    tier: 2,
    rarity: 1,
    performance: 76.8,
    popular: 0.4,
    icon_url: "https://example.com/shrink.png",
    desc: "Reduz o dano do alvo.",
  },
  {
    id: 2062,
    name: "Conjuração Ecoante",
    tier: 0,
    rarity: 8,
    performance: 114.2,
    popular: 3.2,
    icon_url: "https://example.com/echo.png",
    desc: "Conjura habilidades duas vezes.",
  },
  {
    id: 3075,
    name: "Coice",
    tier: 0,
    rarity: 4,
    performance: 99.6,
    popular: 3.8,
    icon_url: "https://example.com/recoil.png",
    desc: "Aumenta o dano e recua.",
  },
];
