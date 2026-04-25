import { en, enUserErrorDetails } from './en';
import { zhCN, zhCNUserErrorDetails } from './zh-CN';
import { zhTW, zhTWUserErrorDetails } from './zh-TW';
import type { Language, RuntimeUserErrorMap, TranslationCopy } from './types';

export const copy: Record<Language, TranslationCopy> = {
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  en,
};

export const userErrorDetails: Record<Language, RuntimeUserErrorMap> = {
  'zh-CN': zhCNUserErrorDetails,
  'zh-TW': zhTWUserErrorDetails,
  en: enUserErrorDetails,
};

export type { CopyKey, Language, UserErrorCode, UserErrorDetail } from './types';
