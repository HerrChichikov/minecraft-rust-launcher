import { invoke, isTauri } from '@tauri-apps/api/core';
export interface AuthSnapshot {
  schemaVersion: 2;
  revision: number;
  persistence: 'memory_only' | 'system_credential_store';
  rememberSupported: boolean;
  providers: {
    id: 'platform' | 'development-mock';
    available: boolean;
    development: boolean;
    flows: string[];
    unavailableReason: string | null;
  }[];
  account: {
    providerId: 'platform' | 'development-mock';
    playerId: string;
    playerName: string;
    development: boolean;
  } | null;
  gameProfiles: GameProfile[];
  selectedGameProfileId: string | null;
  notice:
    | 'registered'
    | 'game_profiles_unavailable'
    | 'remote_logout_failed'
    | 'credential_store_unavailable'
    | 'credential_delete_failed'
    | 'logout_cleanup_failed'
    | null;
}
export interface GameProfile {
  id: string;
  realm: string;
  nickname: string;
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
    v.schemaVersion !== 2 ||
    !Number.isSafeInteger(v.revision) ||
    (v.revision as number) < 0 ||
    (v.persistence !== 'memory_only' && v.persistence !== 'system_credential_store') ||
    typeof v.rememberSupported !== 'boolean' ||
    !Array.isArray(v.providers) ||
    (v.notice !== null &&
      ![
        'registered',
        'game_profiles_unavailable',
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
      (p.id !== 'platform' && p.id !== 'development-mock') ||
      ids.has(p.id) ||
      typeof p.available !== 'boolean' ||
      typeof p.development !== 'boolean' ||
      !Array.isArray(p.flows) ||
      !p.flows.every((f: unknown) => f === 'development_nickname' || f === 'password') ||
      (p.unavailableReason !== null && typeof p.unavailableReason !== 'string')
    )
      throw new AuthError('invalid_response');
    if (
      p.id === 'platform' &&
      (p.development ||
        (p.available ? p.flows.length !== 1 || p.flows[0] !== 'password' : p.flows.length !== 0))
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
  if (!ids.has('platform')) throw new AuthError('invalid_response');
  let account: AuthSnapshot['account'] = null;
  if (v.account !== null) {
    const a = v.account;
    if (
      !record(a) ||
      (a.providerId !== 'platform' && a.providerId !== 'development-mock') ||
      !providers.some((p) => p.id === a.providerId && p.available) ||
      a.development !== (a.providerId === 'development-mock') ||
      typeof a.playerId !== 'string' ||
      !uuid.test(a.playerId) ||
      typeof a.playerName !== 'string' ||
      (a.providerId === 'development-mock'
        ? !/^[A-Za-z0-9_]{3,16}$/.test(a.playerName)
        : a.playerName.length === 0 || [...a.playerName].length > 50)
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
    (!v.rememberSupported || account?.providerId !== 'platform')
  )
    throw new AuthError('invalid_response');
  if (!Array.isArray(v.gameProfiles)) throw new AuthError('invalid_response');
  const gameProfileIds = new Set<string>();
  const gameProfiles: GameProfile[] = v.gameProfiles.map((profile: unknown) => {
    if (
      account?.providerId !== 'platform' ||
      !record(profile) ||
      typeof profile.id !== 'string' ||
      !uuid.test(profile.id) ||
      gameProfileIds.has(profile.id) ||
      typeof profile.realm !== 'string' ||
      !/^[A-Za-z0-9_-]{1,64}$/.test(profile.realm) ||
      typeof profile.nickname !== 'string' ||
      !/^[A-Za-z0-9_]{3,16}$/.test(profile.nickname)
    )
      throw new AuthError('invalid_response');
    gameProfileIds.add(profile.id);
    return { id: profile.id, realm: profile.realm, nickname: profile.nickname };
  });
  const selectedGameProfileId = v.selectedGameProfileId;
  if (
    selectedGameProfileId !== null &&
    (typeof selectedGameProfileId !== 'string' || !gameProfileIds.has(selectedGameProfileId))
  ) {
    throw new AuthError('invalid_response');
  }
  return {
    schemaVersion: 2,
    revision: v.revision as number,
    persistence: v.persistence,
    rememberSupported: v.rememberSupported,
    providers,
    account,
    gameProfiles,
    selectedGameProfileId,
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
      'auth_throttled',
      'game_profile_unavailable',
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
    providerId: 'platform',
    remember,
    input: { flow: 'password', login, password },
  });
export interface Registration {
  nickname: string;
  password: string;
  email: string;
  invitationCode: string;
}
export const registerAccount = (s: AuthSnapshot, fields: Registration) =>
  call('register_account', { expectedRevision: s.revision, ...fields });
export const selectGameProfile = (s: AuthSnapshot, gameProfileId: string) =>
  call('select_game_profile', { expectedRevision: s.revision, gameProfileId });
export const refreshAuth = (s: AuthSnapshot) =>
  call('refresh_auth', { expectedRevision: s.revision });
export const logout = (s: AuthSnapshot) => call('logout', { expectedRevision: s.revision });

export const restoreAuth = (s: AuthSnapshot) =>
  call('restore_auth', { expectedRevision: s.revision });
