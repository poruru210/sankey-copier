import type { NodeTypes } from '@xyflow/react';
import { AccountNode } from './AccountNode';

/**
 * Custom node types for React Flow
 * Defined in a separate file to prevent recreation warnings
 */
export const nodeTypes: NodeTypes = {
  accountNode: AccountNode,
};
