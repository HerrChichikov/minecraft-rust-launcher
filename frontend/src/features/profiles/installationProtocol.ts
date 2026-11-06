export type Status = {
  schemaVersion: 1;
  operationId: string;
  profileId: string;
  profileName: string;
  stage: string;
  sequence: number;
  completedBytes: number;
  totalBytes: number | null;
  errorCode: string | null;
  terminal: boolean;
};
export function parseInstallation(value: unknown): Status | null {
  if (value === null) return null;
  if (typeof value !== 'object' || value === null) throw new Error('invalid_response');
  const v = value as Record<string, unknown>;
  if (
    v.schemaVersion !== 1 ||
    typeof v.operationId !== 'string' ||
    typeof v.profileId !== 'string' ||
    typeof v.profileName !== 'string' ||
    ![
      'initializing',
      'checking',
      'downloading',
      'repairing',
      'extracting',
      'ready',
      'cancelling',
      'cancelled',
      'error',
    ].includes(String(v.stage)) ||
    typeof v.stage !== 'string' ||
    !Number.isSafeInteger(v.sequence) ||
    Number(v.sequence) < 0 ||
    !Number.isSafeInteger(v.completedBytes) ||
    Number(v.completedBytes) < 0 ||
    (v.totalBytes !== null && !Number.isSafeInteger(v.totalBytes)) ||
    typeof v.terminal !== 'boolean' ||
    (v.errorCode !== null && typeof v.errorCode !== 'string')
  )
    throw new Error('invalid_response');
  return v as Status;
}
