import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ChampionBuild,
  ChampSelectedPayload,
  FlashKeyPreference,
  SummonerInfo,
} from "../types";
import { computeOrderedSpells } from "../utils/spells";

export function useLcuEvents(flashKey: FlashKeyPreference) {
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [build, setBuild] = useState<ChampionBuild | null>(null);
  const [spellsInverted, setSpellsInverted] = useState(false);
  const [selectedSecondarySpellId, setSelectedSecondarySpellId] = useState<number | null>(null);
  const [selectedRunePageIndex, setSelectedRunePageIndex] = useState<number>(0);
  const [selectedBuildIndex, setSelectedBuildIndex] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleFilterChange = async (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => {
    if (!build) return;
    const champId = champSelect?.champion_id || build.champion_id;
    const qId = champSelect?.queue_id || 420;

    setLoading(true);
    setErrorMsg(null);
    try {
      const updatedBuild = await invoke<ChampionBuild>("update_build_filters", {
        championId: champId,
        queueId: qId,
        position: filterType === "position" ? value : (build.active_position || null),
        tier: filterType === "tier" ? value : (build.active_tier || null),
        region: filterType === "region" ? value : (build.active_region || null),
        patch: filterType === "patch" ? value : (build.active_patch || null),
      });
      if (updatedBuild) {
        setBuild(updatedBuild);
      }
    } catch (err: unknown) {
      console.error("Falha ao atualizar filtros da build:", err);
      setErrorMsg(
        typeof err === "string"
          ? err
          : "Erro ao carregar build com os filtros selecionados."
      );
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Initial check for summoner status
    invoke<SummonerInfo | null>("get_summoner_status")
      .then((data) => {
        if (data) setSummoner(data);
      })
      .catch(() => {});

    // Listen to real-time events emitted from Tauri backend
    let unlistenConnected: (() => void) | undefined;
    let unlistenDisconnected: (() => void) | undefined;
    let unlistenChampSelected: (() => void) | undefined;
    let unlistenChampSelectEnded: (() => void) | undefined;
    let unlistenBuild: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenConnected = await listen<SummonerInfo>("lcu-connected", (event) => {
          setSummoner(event.payload);
          setErrorMsg(null);
        });

        unlistenDisconnected = await listen("lcu-disconnected", () => {
          setSummoner(null);
          setChampSelect(null);
          setBuild(null);
        });

        unlistenChampSelected = await listen<ChampSelectedPayload>("champ-selected", async (event) => {
          setChampSelect((prev) => {
            if (prev?.champion_id !== event.payload.champion_id) {
              setSpellsInverted(false);
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return event.payload;
          });

          // Fetch recommended build for selected champion if not already received
          try {
            if (event.payload.assigned_position) {
              const recommended = await invoke<ChampionBuild>("update_build_filters", {
                championId: event.payload.champion_id,
                queueId: event.payload.queue_id || 420,
                position: event.payload.assigned_position,
                tier: null,
                region: null,
                patch: null,
              });
              if (recommended) {
                setBuild((prev) => {
                  if (prev?.champion_id !== recommended.champion_id) {
                    setSelectedSecondarySpellId(null);
                    setSelectedRunePageIndex(0);
                    setSelectedBuildIndex(0);
                  }
                  return recommended;
                });
              }
            } else {
              const recommended = await invoke<ChampionBuild>("get_recommended_build_for_champion", {
                championId: event.payload.champion_id,
                queueId: event.payload.queue_id || 420,
              });
              if (recommended) {
                setBuild((prev) => {
                  if (prev?.champion_id !== recommended.champion_id) {
                    setSelectedSecondarySpellId(null);
                    setSelectedRunePageIndex(0);
                    setSelectedBuildIndex(0);
                  }
                  return recommended;
                });
              }
            }
          } catch {
            // Ignore background fetch error, ws_listener will emit recommended-build
          }
        });

        unlistenChampSelectEnded = await listen("champ-select-ended", () => {
          setChampSelect(null);
          setBuild(null);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);
          setSelectedRunePageIndex(0);
          setSelectedBuildIndex(0);
        });

        unlistenBuild = await listen<ChampionBuild>("recommended-build", (event) => {
          setBuild((prev) => {
            if (prev?.champion_id !== event.payload.champion_id) {
              setSelectedSecondarySpellId(null);
              setSelectedRunePageIndex(0);
              setSelectedBuildIndex(0);
            }
            return event.payload;
          });
        });
      } catch (e) {
        console.warn("Tauri event listener setup skipped or running in test mode:", e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenConnected) unlistenConnected();
      if (unlistenDisconnected) unlistenDisconnected();
      if (unlistenChampSelected) unlistenChampSelected();
      if (unlistenChampSelectEnded) unlistenChampSelectEnded();
      if (unlistenBuild) unlistenBuild();
    };
  }, []);

  const orderedSpells = computeOrderedSpells(
    build?.spells,
    selectedSecondarySpellId,
    build?.spell_choices,
    flashKey,
    spellsInverted
  );

  const handleSelectSecondarySpell = async (spellId: number) => {
    setSelectedSecondarySpellId(spellId);
    const reordered = computeOrderedSpells(
      build?.spells,
      spellId,
      build?.spell_choices,
      flashKey,
      spellsInverted
    );
    if (reordered.length >= 2) {
      try {
        await invoke("inject_summoner_spells", {
          spell1Id: reordered[0].id,
          spell2Id: reordered[1].id,
          championId: build?.champion_id,
          secondarySpellId: spellId,
        });
      } catch (e) {
        console.warn("Failed to inject secondary spell change:", e);
      }
    }
  };

  const handleSelectRunePage = async (pageIndex: number) => {
    if (!build?.rune_pages || pageIndex >= build.rune_pages.length) return;
    setSelectedRunePageIndex(pageIndex);
    const page = build.rune_pages[pageIndex];
    try {
      await invoke("inject_rune_page", {
        championId: build.champion_id,
        championName: build.champion_name,
        pageIndex,
        primaryStyleId: page.primary_style_id,
        subStyleId: page.sub_style_id,
        selectedPerkIds: page.selected_perk_ids,
      });
    } catch (e) {
      console.warn("Failed to inject chosen rune page:", e);
    }
  };

  const handleSelectBuild = async (index: number) => {
    if (!build?.build_options || index >= build.build_options.length) return;
    setSelectedBuildIndex(index);
    try {
      await invoke("inject_item_build", {
        championId: build.champion_id,
        buildIndex: index,
        coreIndex: index,
        finalIndex: index,
      });
    } catch (e) {
      console.warn("Failed to inject chosen unified build:", e);
    }
  };

  return {
    summoner,
    champSelect,
    build,
    spellsInverted,
    setSpellsInverted,
    selectedSecondarySpellId,
    selectedRunePageIndex,
    selectedBuildIndex,
    loading,
    errorMsg,
    orderedSpells,
    handleFilterChange,
    handleSelectSecondarySpell,
    handleSelectRunePage,
    handleSelectBuild,
  };
}
