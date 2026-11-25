import { useEffect, useRef, useState } from "react";
import { useTranslation } from "../../i18n";
import { AramAugmentEntry } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ListSkeleton, LoadingDots } from "../../components/ui/LoadingIndicators";
import { SearchInput } from "../../components/ui/SearchInput";
import {
  AugmentRarityFilter,
  getAugmentRarityInfo,
  getAugmentTierClass,
  getAugmentTierLabel,
} from "./aramHelpers";

const MAX_ICON_RETRIES = 2;
const ICON_RETRY_DELAYS_MS = [400, 1000] as const;

function getAugmentInitials(name: string): string {
  return name.trim().slice(0, 2).toUpperCase() || "?";
}

interface AugmentIconProps {
  iconUrl: string;
  name: string;
  rarityClassName: string;
}

function AugmentIcon({ iconUrl, name, rarityClassName }: AugmentIconProps) {
  const [retryCount, setRetryCount] = useState(0);
  const [isWaitingRetry, setIsWaitingRetry] = useState(false);
  const [hasFailed, setHasFailed] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resolvedUrl = resolveCachedAssetUrl(iconUrl);

  useEffect(() => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setRetryCount(0);
    setIsWaitingRetry(false);
    setHasFailed(false);
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [iconUrl]);

  const handleError = () => {
    if (retryCount < MAX_ICON_RETRIES) {
      const delayMs = ICON_RETRY_DELAYS_MS[retryCount] ?? 1000;
      setIsWaitingRetry(true);
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        timerRef.current = null;
        setRetryCount((prev) => prev + 1);
        setIsWaitingRetry(false);
      }, delayMs);
    } else {
      setHasFailed(true);
    }
  };

  const srcWithRetry =
    retryCount === 0
      ? resolvedUrl
      : `${resolvedUrl}${resolvedUrl.includes("?") ? "&" : "?"}retry=${retryCount}`;

  return (
    <div className={`aram-augment-icon-frame ${rarityClassName}`}>
      {!resolvedUrl || hasFailed ? (
        <span className="aram-icon-fallback" data-testid="aram-augment-icon-fallback">
          {getAugmentInitials(name)}
        </span>
      ) : isWaitingRetry ? (
        <span className="aram-icon-fallback" aria-hidden="true" />
      ) : (
        <img
          src={srcWithRetry}
          alt={name}
          className="aram-augment-icon"
          loading="lazy"
          onError={handleError}
        />
      )}
    </div>
  );
}

interface AramMayhemAugmentsPanelProps {
  rarityFilter: AugmentRarityFilter;
  onSelectRarityFilter: (rarity: AugmentRarityFilter) => void;
  augmentSearch: string;
  onAugmentSearchChange: (val: string) => void;
  augmentsLoading: boolean;
  augmentsError: string | null;
  currentAugments: AramAugmentEntry[];
  onRetryAugments: () => void;
}

export function AramMayhemAugmentsPanel({
  rarityFilter,
  onSelectRarityFilter,
  augmentSearch,
  onAugmentSearchChange,
  augmentsLoading,
  augmentsError,
  currentAugments,
  onRetryAugments,
}: AramMayhemAugmentsPanelProps) {
  const { t } = useTranslation();

  const rarityFilterOptions = [
    { id: "all" as const, label: t("aram.rarityAll"), dot: "" },
    { id: 1 as const, label: t("aram.raritySilver"), dot: "dot-silver" },
    { id: 4 as const, label: t("aram.rarityGold"), dot: "dot-gold" },
    { id: 8 as const, label: t("aram.rarityPrismatic"), dot: "dot-prismatic" },
  ];

  return (
    <>
      <div className="aram-augments-controls">
        <div
          className="segmented-control aram-rarity-filter"
          role="group"
          aria-label={t("aram.filterRarityAria")}
        >
          {rarityFilterOptions.map((opt) => (
            <button
              key={String(opt.id)}
              type="button"
              className={`segment-btn ${rarityFilter === opt.id ? "active" : ""}`}
              onClick={() => onSelectRarityFilter(opt.id)}
            >
              {opt.dot && <span className={`aram-rarity-dot ${opt.dot}`} />}
              <span>{opt.label}</span>
            </button>
          ))}
        </div>

        <SearchInput
          value={augmentSearch}
          onChange={onAugmentSearchChange}
          placeholder={t("aram.searchAugmentPlaceholder")}
          ariaLabel={t("aram.searchAugmentAria")}
          clearAriaLabel={t("aram.clearSearchAugmentAria")}
          className="aram-aug-search"
        />
      </div>

      <div className="aram-scroll-body" data-testid="aram-augments-list">
        {augmentsLoading ? (
          <div className="loading-skeleton-shell" data-testid="aram-augments-loading">
            <div className="loading-banner">
              <div className="spinner aram-spinner" />
              <span>{t("aram.loadingAugments")}</span>
              <LoadingDots size="sm" testId="aram-augments-loading-dots" />
            </div>
            <ListSkeleton rows={5} variant="augments" testId="aram-augments-skeleton" />
          </div>
        ) : augmentsError ? (
          <div className="aram-state-box">
            <span className="aram-error-text">{augmentsError}</span>
            <button type="button" className="aram-retry-btn" onClick={onRetryAugments}>
              {t("common.retry")}
            </button>
          </div>
        ) : currentAugments.length === 0 ? (
          <div className="aram-state-box">
            <span>{t("aram.noAugmentsFound")}</span>
          </div>
        ) : (
          currentAugments.map((aug) => {
            const rarityInfo = getAugmentRarityInfo(aug.rarity, t);
            return (
              <div key={aug.id} className="aram-augment-row">
                <AugmentIcon
                  iconUrl={aug.icon_url}
                  name={aug.name}
                  rarityClassName={rarityInfo.className}
                />

                <span className={`aram-aug-tier-badge ${getAugmentTierClass(aug.tier)}`}>
                  {getAugmentTierLabel(aug.tier)}
                </span>

                <div className="aram-augment-info">
                  <div className="aram-augment-top">
                    <strong className="aram-augment-name">{aug.name}</strong>
                    <span className={`aram-rarity-tag ${rarityInfo.className}`}>
                      {rarityInfo.label}
                    </span>
                    {(aug.popular ?? 0) > 0 && (
                      <span className="aram-aug-pick">
                        {t("aram.augmentPickRate", { rate: (aug.popular ?? 0).toFixed(1) })}
                      </span>
                    )}
                  </div>
                  {aug.desc && <p className="aram-augment-desc">{aug.desc}</p>}
                </div>
              </div>
            );
          })
        )}
      </div>
    </>
  );
}
