export interface BuildTranslations {
  runes: {
    mayhemBadge: string;
    mayhemDisabledTitle: string;
    mayhemDisabledPrefix: string;
    mayhemDisabledMode: string;
    mayhemDisabledMiddle: string;
    mayhemDisabledAugments: string;
    mayhemDisabledSuffix: string;
    mayhemInfoSpells: string;
    mayhemInfoItems: string;
    primaryTreeFallback: string;
    subTreeFallback: string;
    pageMostUsed: string;
    pageNumber: string;
    optionMostUsed: string;
    optionNumber: string;
    keystoneFallback: string;
    secondaryFallback: string;
    statShardsTitle: string;
    shards: {
      adaptive: string;
      attackSpeed: string;
      abilityHaste: string;
      armor: string;
      magicResist: string;
      healthScaling: string;
      moveSpeed: string;
      health: string;
      tenacity: string;
    };
  };
  spells: {
    title: string;
    invertBtn: string;
    invertTitle: string;
    spell1Fallback: string;
    spell2Fallback: string;
    secondaryReadOnlyTitle: string;
    secondaryEditTitle: string;
    mostUsed: string;
    alternative: string;
    secondMostUsed: string;
  };
  items: {
    titleReadOnly: string;
    titleShop: string;
    usageLabel: string;
    pickLabel: string;
    coreTrioTag: string;
    itemsCountTag: string;
    optionalTag: string;
    blockStarter: string;
    blockCore: string;
    blockBoots: string;
    blockFinal: string;
    blockSituational: string;
    build1MostPopular: string;
    buildOptionNumber: string;
  };
}
