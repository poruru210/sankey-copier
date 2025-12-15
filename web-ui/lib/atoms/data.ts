import { atom } from 'jotai';
import type { EaConnection, CopySettings, TradeGroup } from '@/types';

// Raw data atoms
export const connectionsAtom = atom<EaConnection[]>([]);
export const settingsAtom = atom<CopySettings[]>([]);
export const tradeGroupsAtom = atom<TradeGroup[]>([]);

export const localizationAtom = atom({
  allSourcesInactive: 'All sources inactive',
  someSourcesInactive: 'Some sources inactive',
  autoTradingDisabled: 'Auto trading disabled',
});
