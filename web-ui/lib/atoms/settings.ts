import { atom } from 'jotai';
import { CopySettings } from '@/types';

export const settingsAtom = atom<CopySettings[]>([]);
