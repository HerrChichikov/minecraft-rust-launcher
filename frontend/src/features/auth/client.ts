import { invoke, isTauri } from '@tauri-apps/api/core';

export interface AuthSnapshot {
  schemaVersion: 1;
  revision: number;
  persistence: 'memory_only';
  providers: {
    id: 'ely-by' | 'development-mock';
    available: boolean;
    development: boolean;
    flows: string[];
    unavailableReason: string | null;
  }[];
  account: {
    providerId: 'development-mock';
    playerId: string;
    playerName: string;
    development: true;
  } | null;
}
const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
export class AuthError extends Error {
  constructor(readonly code: string) {
    super(code);
  }
}
export function parseAuth(v: unknown): AuthSnapshot {
  if (
    !record(v) ||
    v.schemaVersion !== 1 ||
    !Number.isSafeInteger(v.revision) ||
    (v.revision as number) < 0 ||
    v.persistence !== 'memory_only' ||
    !Array.isArray(v.providers)
  )
    throw new AuthError('invalid_response');
  const ids = new Set<string>();
  const providers: AuthSnapshot['providers'] = v.providers.map((p: unknown) => {
    if (
      !record(p) ||
      (p.id !== 'ely-by' && p.id !== 'development-mock') ||
      ids.has(p.id) ||
      typeof p.available !== 'boolean' ||
      typeof p.development !== 'boolean' ||
      !Array.isArray(p.flows) ||
      !p.flows.every((f: unknown) => f === 'development_nickname') ||
      (p.unavailableReason !== null && typeof p.unavailableReason !== 'string')
    )
      throw new AuthError('invalid_response');
    if (
      (p.id === 'ely-by' && (p.available || p.development || p.flows.length !== 0)) ||
      (p.id === 'development-mock' && (!p.available || !p.development || p.flows.length !== 1))
    )
      throw new AuthError('invalid_response');
    ids.add(p.id);
    return {
      id: p.id,
      available: p.available,
      development: p.development,
      flows: p.flows,
      unavailableReason: p.unavailableReason,
    };
  });
  if (!ids.has('ely-by')) throw new AuthError('invalid_response');
  let account: AuthSnapshot['account'] = null;
  if (v.account !== null) {
    const a = v.account;
    if (
      !record(a) ||
      a.providerId !== 'development-mock' ||
      !ids.has(a.providerId) ||
      a.development !== true ||
      typeof a.playerId !== 'string' ||
      !uuid.test(a.playerId) ||
      typeof a.playerName !== 'string' ||
      !/^[A-Za-z0-9_]{3,16}$/.test(a.playerName)
    )
      throw new AuthError('invalid_response');
    account = {
      providerId: a.providerId,
      playerId: a.playerId,
      playerName: a.playerName,
      development: true,
    };
  }
  return {
    schemaVersion: 1,
    revision: v.revision as number,
    persistence: 'memory_only',
    providers,
    account,
  };
}
async function call(command: string, request?: object): Promise<AuthSnapshot> {
  if (!isTauri()) throw new AuthError('desktop_required');
  let value: unknown;
  try {
    value = await invoke<unknown>(command, request ? { request } : undefined);
  } catch (error: unknown) {
    const codes = ['auth_unavailable', 'revision_conflict', 'invalid_settings', 'storage_busy'];
    throw new AuthError(
      record(error) && typeof error.code === 'string' && codes.includes(error.code)
        ? error.code
        : 'auth_failed',
    );
  }
  return parseAuth(value);
}
export const getAuth = () => call('get_auth');
export const login = (s: AuthSnapshot, nickname: string) =>
  call('login', { expectedRevision: s.revision, providerId: 'development-mock', nickname });
export const logout = (s: AuthSnapshot) => call('logout', { expectedRevision: s.revision });
