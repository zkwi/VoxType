import type { AppConfig, ConfigValidationError } from "$lib/types/app";

export function clonePlain<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function configFingerprint(value: AppConfig) {
  return JSON.stringify(value);
}

export function validationErrorMap(errors: ConfigValidationError[]) {
  return Object.fromEntries(
    errors
      .filter((error) => error.field && error.message)
      .map((error) => [error.field, error.message]),
  );
}

export function firstValidationField(errors: ConfigValidationError[]) {
  return errors.find((error) => error.field)?.field ?? "";
}
