import { call } from './invoke';
import type { CardInfo } from './types';

export const listCards = () => call<CardInfo[]>('card_list');
export const selectCard = (path: string) => call<CardInfo>('card_select', { path });
export const currentCard = () => call<CardInfo | null>('card_current');
export const clearCard = () => call<null>('card_clear');
