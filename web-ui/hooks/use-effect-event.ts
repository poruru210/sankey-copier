import { useRef, useLayoutEffect, useCallback } from 'react';

/**
 * A shim for the experimental React.useEffectEvent hook.
 * This hook allows you to define a function that has stable identity but accesses the latest props/state.
 * It should ONLY be called from within useEffect, useLayoutEffect, or useInsertionEffect.
 * Do NOT pass the returned function as a prop to child components.
 */
export function useEffectEvent<T extends (...args: any[]) => any>(fn: T): T {
    const ref = useRef(fn);

    useLayoutEffect(() => {
        ref.current = fn;
    }, [fn]);

    return useCallback((...args: any[]) => {
        return ref.current(...args);
    }, []) as T;
}
