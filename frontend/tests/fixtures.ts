import contract from '../../tests/contracts/foundation-snapshot.v1.json';
import { parseSnapshot } from '../src/shared/ipc/client';

export const foundationSnapshot = parseSnapshot(contract);
