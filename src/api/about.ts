import { call } from './invoke';

/** The licence notices of the app's dependencies, or null in a build without them. */
export const aboutNotices = () => call<string | null>('about_notices');
