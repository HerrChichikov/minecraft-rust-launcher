export type OperatingSystem = 'windows' | 'linux' | 'macos';
export type Architecture = 'x86_64' | 'aarch64';

export interface FoundationSnapshot {
  schemaVersion: 1;
  phase: 'foundation';
  launcherVersion: string;
  targetMinecraftVersion: string;
  platform: {
    os: OperatingSystem;
    architecture: Architecture;
  };
  developmentBuild: boolean;
  features: {
    profiles: boolean;
    installation: boolean;
    authentication: boolean;
    gameLaunch: boolean;
  };
}

export type ClientErrorCode =
  'desktop_required' | 'backend_unavailable' | 'unsupported_platform' | 'invalid_response';
