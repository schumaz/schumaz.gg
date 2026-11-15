import { PlayerProfile, SummonerInfo } from "../../../types";

export const mockSummoner: SummonerInfo = {
  summonerId: 987654321,
  gameName: "Schuma",
  tagLine: "BR1",
  displayName: "Schuma",
  summonerLevel: 350,
  profileIconId: 548,
};

export const mockProfile: PlayerProfile = {
  summoner: mockSummoner,
  profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/548.png",
  background_skin_id: 103015,
  background_skin_name: "Ahri Florescer Espiritual",
  background_splash_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg",
  background_splash_fallback_url: "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg",
  ranked_stats: [
    {
      queue_type: "RANKED_SOLO_5x5",
      queue_label: "Solo/Duo",
      tier: "EMERALD",
      tier_label: "Esmeralda",
      division: "II",
      league_points: 67,
      wins: 52,
      losses: 44,
    },
    {
      queue_type: "RANKED_FLEX_SR",
      queue_label: "Flex",
      tier: "UNRANKED",
      tier_label: "Sem Ranque",
      division: "",
      league_points: 0,
      wins: 0,
      losses: 0,
    },
  ],
  top_masteries: [
    {
      champion_id: 103,
      champion_name: "Ahri",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png",
      champion_level: 25,
      champion_points: 452310,
    },
    {
      champion_id: 238,
      champion_name: "Zed",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png",
      champion_level: 18,
      champion_points: 310400,
    },
    {
      champion_id: 157,
      champion_name: "Yasuo",
      champion_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Yasuo.png",
      champion_level: 12,
      champion_points: 185000,
    },
  ],
  recent_matches: [
    ...Array.from({ length: 10 }, (_, idx) => {
      const id = 1000 + idx;
      const isSolo = idx < 5;
      const isAram = idx >= 5 && idx < 8;
      const win = isSolo ? idx !== 2 : isAram ? idx === 5 : idx === 8;
      return {
        game_id: id,
        queue_id: isSolo ? 420 : isAram ? 450 : 440,
        game_mode_key: isSolo ? "ranked_solo" : isAram ? "aram" : "ranked_flex",
        game_mode_label: isSolo ? "Solo/Duo" : isAram ? "ARAM" : "Flex",
        champion_id: isSolo ? 103 : 238,
        champion_name: isSolo ? "Ahri" : "Zed",
        champion_icon_url: isSolo
          ? "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
          : "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Zed.png",
        win,
        kills: 9,
        deaths: 3,
        assists: 7,
        cs: 195,
        game_duration_secs: 1800,
        game_creation_ms: Date.now() - (idx + 1) * 3600000,
        spell1_id: 4,
        spell1_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png",
        spell2_id: 14,
        spell2_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png",
        item_ids: [6655, 3020, 3089, 3157, 0, 0],
        item_icon_urls: [
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/6655.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3020.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3089.png",
          "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/item/3157.png",
          "",
          "",
        ],
        gold_earned: 14500,
        total_damage_dealt_to_champions: 29800,
        vision_score: 22,
        wards_placed: 9,
        champ_level: 16,
        double_kills: idx === 0 ? 1 : 0,
        triple_kills: 0,
        quadra_kills: 0,
        penta_kills: 0,
      };
    }),
  ],
};
