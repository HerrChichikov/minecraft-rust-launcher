import { invoke, isTauri } from '@tauri-apps/api/core';
export interface AuthSnapshot {
  schemaVersion: 1;
  revision: number;
  persistence: 'memory_only' | 'system_credential_store';
  rememberSupported: boolean;
  providers: {
    id: 'ely-by' | 'development-mock';
    available: boolean;
    development: boolean;
    flows: string[];
    unavailableReason: string | null;
  }[];
  account: {
    providerId: 'ely-by' | 'development-mock';
    playerId: string;
    playerName: string;
    development: boolean;
  } | null;
  challenge: { id: string; providerId: 'ely-by'; expiresInSeconds: number } | null;
  notice:
    | 'remote_logout_failed'
    | 'credential_store_unavailable'
    | 'credential_delete_failed'
    | 'logout_cleanup_failed'
    | null;
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
    (v.persistence !== 'memory_only' && v.persistence !== 'system_credential_store') ||
    typeof v.rememberSupported !== 'boolean' ||
    !Array.isArray(v.providers) ||
    (v.notice !== null &&
      ![
        'remote_logout_failed',
        'credential_store_unavailable',
        'credential_delete_failed',
        'logout_cleanup_failed',
      ].includes(v.notice as string))
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
      !p.flows.every((f: unknown) => f === 'development_nickname' || f === 'password_totp') ||
      (p.unavailableReason !== null && typeof p.unavailableReason !== 'string')
    )
      throw new AuthError('invalid_response');
    if (
      p.id === 'ely-by' &&
      (p.development ||
        (p.available
          ? p.flows.length !== 1 || p.flows[0] !== 'password_totp'
          : p.flows.length !== 0))
    )
      throw new AuthError('invalid_response');
    if (
      p.id === 'development-mock' &&
      (!p.available ||
        !p.development ||
        p.flows.length !== 1 ||
        p.flows[0] !== 'development_nickname')
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
      (a.providerId !== 'ely-by' && a.providerId !== 'development-mock') ||
      !ids.has(a.providerId) ||
      a.development !== (a.providerId === 'development-mock') ||
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
      development: a.providerId === 'development-mock',
    };
  }
  if (
    v.persistence === 'system_credential_store' &&
    (!v.rememberSupported || account?.providerId !== 'ely-by')
  )
    throw new AuthError('invalid_response');
  let challenge: AuthSnapshot['challenge'] = null;
  if (v.challenge !== null) {
    const c = v.challenge;
    if (
      !record(c) ||
      typeof c.id !== 'string' ||
      !uuid.test(c.id) ||
      c.providerId !== 'ely-by' ||
      typeof c.expiresInSeconds !== 'number' ||
      !Number.isSafeInteger(c.expiresInSeconds) ||
      c.expiresInSeconds < 0 ||
      c.expiresInSeconds > 120
    )
      throw new AuthError('invalid_response');
    challenge = { id: c.id, providerId: 'ely-by', expiresInSeconds: c.expiresInSeconds };
  }
  return {
    schemaVersion: 1,
    revision: v.revision as number,
    persistence: v.persistence,
    rememberSupported: v.rememberSupported,
    providers,
    account,
    challenge,
    notice: v.notice as AuthSnapshot['notice'],
  };
}
async function call(command: string, request?: object): Promise<AuthSnapshot> {
  if (!isTauri()) throw new AuthError('desktop_required');
  let value: unknown;
  try {
    value = await invoke<unknown>(command, request ? { request } : undefined);
  } catch (error: unknown) {
    const codes = [
      'auth_unavailable',
      'revision_conflict',
      'invalid_settings',
      'auth_busy',
      'auth_rejected',
      'auth_network',
      'auth_protocol',
      'auth_challenge_expired',
      'authentication_required',
      'unsupported_auth_flow',
      'credential_store_unavailable',
      'saved_session_missing',
    ];
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
  call('login', {
    expectedRevision: s.revision,
    providerId: 'development-mock',
    input: { flow: 'development_nickname', nickname },
  });
export const loginPassword = (s: AuthSnapshot, login: string, password: string, remember = false) =>
  call('login', {
    expectedRevision: s.revision,
    providerId: 'ely-by',
    remember,
    input: { flow: 'password', login, password },
  });
export const continueLogin = (s: AuthSnapshot, totp: string) =>
  call('continue_login', { expectedRevision: s.revision, challengeId: s.challenge?.id, totp });
export const cancelLogin = (s: AuthSnapshot) =>
  call('cancel_login', { expectedRevision: s.revision });
export const refreshAuth = (s: AuthSnapshot) =>
  call('refresh_auth', { expectedRevision: s.revision });
export const logout = (s: AuthSnapshot) => call('logout', { expectedRevision: s.revision });

export const restoreAuth = (s: AuthSnapshot) =>
  call('restore_auth', { expectedRevision: s.revision });
