import contract from '../../tests/contracts/foundation-snapshot.v1.json';
import { parseSnapshot } from '../src/shared/ipc/client';

export const foundationSnapshot = parseSnapshot(contract);

import workspaceContract from '../../tests/contracts/workspace.v1.json';
import { parseWorkspace } from '../src/features/profiles/client';
export const workspaceFixture = parseWorkspace(workspaceContract);
