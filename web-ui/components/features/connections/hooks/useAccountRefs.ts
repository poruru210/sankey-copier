import { useRef, useCallback } from 'react';

interface UseAccountRefsReturn {
  sourceRefs: React.MutableRefObject<{ [key: string]: HTMLDivElement | null }>;
  receiverRefs: React.MutableRefObject<{ [key: string]: HTMLDivElement | null }>;
  registerSourceRef: (accountId: string) => (el: HTMLDivElement | null) => void;
  registerReceiverRef: (accountId: string) => (el: HTMLDivElement | null) => void;
}

/**
 * Custom hook for managing DOM refs for source and receiver account elements
 * Used for SVG connection line positioning
 *
 * @returns Ref objects and registration functions
 */
export function useAccountRefs(): UseAccountRefsReturn {
  const sourceRefs = useRef<{ [key: string]: HTMLDivElement | null }>({});
  const receiverRefs = useRef<{ [key: string]: HTMLDivElement | null }>({});

  // Create a stable ref registration function for sources
  const registerSourceRef = useCallback(
    (accountId: string) => (el: HTMLDivElement | null) => {
      if (el) {
        sourceRefs.current[accountId] = el;
      }
    },
    []
  );

  // Create a stable ref registration function for receivers
  const registerReceiverRef = useCallback(
    (accountId: string) => (el: HTMLDivElement | null) => {
      if (el) {
        receiverRefs.current[accountId] = el;
      }
    },
    []
  );

  return {
    sourceRefs,
    receiverRefs,
    registerSourceRef,
    registerReceiverRef,
  };
}
