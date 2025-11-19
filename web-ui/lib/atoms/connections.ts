import { atom } from 'jotai';
import { EaConnection } from '@/types';

export const connectionsAtom = atom<EaConnection[]>([]);
