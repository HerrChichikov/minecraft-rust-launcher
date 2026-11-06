import { invoke, isTauri } from '@tauri-apps/api/core';

import type { ClientErrorCode, FoundationSnapshot } from './types';

export class LauncherClientError extends Error {
  readonly code: ClientErrorCode;

  constructor(code: ClientErrorCode) {
    super(code);
    this.name = 'LauncherClientError';
    this.code = code;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function parseSnapshot(value: unknown): FoundationSnapshot {
  if (
    !isRecord(value) ||
    value.schemaVersion !== 1 ||
    value.phase !== 'foundation' ||
    typeof value.launcherVersion !== 'string' ||
    typeof value.targetMinecraftVersion !== 'string' ||
    typeof value.developmentBuild !== 'boolean'
  ) {
    throw new LauncherClientError('invalid_response');
  }

  const { platform, features } = value;
  if (
    !isRecord(platform) ||
    (platform.os !== 'windows' && platform.os !== 'linux' && platform.os !== 'macos') ||
    (platform.architecture !== 'x86_64' && platform.architecture !== 'aarch64') ||
    !isRecord(features) ||
    typeof features.profiles !== 'boolean' ||
    typeof features.installation !== 'boolean' ||
    typeof features.authentication !== 'boolean' ||
    typeof features.gameLaunch !== 'boolean'
  ) {
    throw new LauncherClientError('invalid_response');
  }

  return {
    schemaVersion: 1,
    phase: 'foundation',
    launcherVersion: value.launcherVersion,
    targetMinecraftVersion: value.targetMinecraftVersion,
    platform: { os: platform.os, architecture: platform.architecture },
    developmentBuild: value.developmentBuild,
    features: {
      profiles: features.profiles,
      installation: features.installation,
      authentication: features.authentication,
      gameLaunch: features.gameLaunch,
    },
  };
}

export async function getSnapshot(): Promise<FoundationSnapshot> {
  if (!isTauri()) {
    throw new LauncherClientError('desktop_required');
  }

  let response: unknown;
  try {
    response = await invoke<unknown>('get_snapshot');
  } catch (error: unknown) {
    if (isRecord(error) && error.code === 'unsupported_platform') {
      throw new LauncherClientError('unsupported_platform');
    }
    // Never pass raw Rust errors, paths, or HTTP responses to the UI.
    throw new LauncherClientError('backend_unavailable');
  }
  return parseSnapshot(response);
}
