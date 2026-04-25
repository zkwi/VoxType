import type { zhCN } from './zh-CN';

export type Language = 'zh-CN' | 'zh-TW' | 'en';

type WidenStrings<T> = {
  [K in keyof T]: T[K] extends string ? string : WidenStrings<T[K]>;
};

export type TranslationCopy = WidenStrings<typeof zhCN>;
export type CopyKey = keyof TranslationCopy;

export type {
  RuntimeUserErrorMap,
  UserErrorCode,
  UserErrorDetail,
  UserErrorMap,
} from './errorCodes';
