import { useOperation } from '../../shared/ipc/useOperation';
import { parseInstallation } from './installationProtocol';

export function useInstallation() {
  const channel = useOperation({
    event: 'launcher://installation',
    snapshot: 'get_installation',
    parse: parseInstallation,
    connectionError: 'Не удалось подключиться к установщику.',
  });
  return {
    ...channel,
    active: channel.pending || Boolean(channel.status && !channel.status.terminal),
  };
}
