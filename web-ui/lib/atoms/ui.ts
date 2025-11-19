import { atom } from 'jotai';

// Hover states
export const hoveredSourceIdAtom = atom<string | null>(null);
export const hoveredReceiverIdAtom = atom<string | null>(null);

// Selection states
export const selectedSourceIdAtom = atom<string | null>(null);
export const selectedMasterAtom = atom<string>('all');

// Expanded states
export const expandedSourceIdsAtom = atom<string[]>([]);
export const expandedReceiverIdsAtom = atom<string[]>([]);

// Disabled states (for sources which don't have direct settings)
export const disabledSourceIdsAtom = atom<string[]>([]);
export const disabledReceiverIdsAtom = atom<string[]>([]);
