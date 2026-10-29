import { call } from './invoke';
import type { CardInfo } from './types';

export const listCards = () => call<CardInfo[]>('card_list');
export const selectCard = (path: string) => call<CardInfo>('card_select', { path });
export const currentCard = () => call<CardInfo | null>('card_current');
export const clearCard = () => call<null>('card_clear');
/** Whether the selected player's folder can still be opened (a drive picked by hand is not listed). */
export const cardPresent = () => call<boolean>('card_present');

/** Deselects the player at `root` in the backend if its folder cannot be opened; true if it did. */
export const cardForgetIfMissing = (root: string) =>
  call<boolean>('card_forget_if_missing', { root });
