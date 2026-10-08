let adminApiKey: string | null = null;
let authEpoch = 0;

export function getAdminApiKey(): string | null {
  return adminApiKey;
}

export function getAdminAuthEpoch(): number {
  return authEpoch;
}

export function setAdminApiKey(key: string): void {
  adminApiKey = key;
  authEpoch += 1;
}

export function clearAdminApiKey(): void {
  adminApiKey = null;
  authEpoch += 1;
}
