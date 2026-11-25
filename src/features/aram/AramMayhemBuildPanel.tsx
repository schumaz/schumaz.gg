import { useTranslation } from "../../i18n";
import { ChampionBuild, FlashKeyPreference, SpellMetadata } from "../../types";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { LoadingDots } from "../../components/ui/LoadingIndicators";
import { SpellsBar } from "../../components/ui/SpellsBar";

interface AramMayhemBuildPanelProps {
  championName: string;
  build: ChampionBuild | null;
  loading: boolean;
  error: string | null;
  orderedSpells: SpellMetadata[];
  flashKey?: FlashKeyPreference;
  selectedBuildIndex: number;
  onSelectBuild: (index: number) => void;
  onRetryBuild: () => void;
}

/**
 * Read-only ARAM Mayhem build consultation panel showing Summoner Spells and the 5 item blocks.
 */
export function AramMayhemBuildPanel({
  championName,
  build,
  loading,
  error,
  orderedSpells,
  flashKey,
  selectedBuildIndex,
  onSelectBuild,
  onRetryBuild,
}: AramMayhemBuildPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="aram-mayhem-build-body" data-testid="aram-mayhem-build-panel">
      {loading && !build ? (
        <div className="aram-state-box" data-testid="aram-mayhem-build-loading">
          <div className="loading-status-line">
            <div className="spinner aram-spinner" />
            <span>{t("aram.loadingMayhemBuild", { name: championName })}</span>
            <LoadingDots size="sm" testId="aram-mayhem-build-loading-dots" />
          </div>
        </div>
      ) : error && !build ? (
        <div className="aram-state-box">
          <span className="aram-error-text">{error}</span>
          <button type="button" className="aram-retry-btn" onClick={onRetryBuild}>
            {t("common.retry")}
          </button>
        </div>
      ) : build ? (
        <>
          <SpellsBar
            orderedSpells={orderedSpells}
            spellOptions={build.spell_options}
            flashKey={flashKey}
            readOnly={true}
          />
          <ItemBlocks
            build={build}
            selectedBuildIndex={selectedBuildIndex}
            onSelectBuild={onSelectBuild}
            readOnly={true}
          />
        </>
      ) : null}
    </div>
  );
}
