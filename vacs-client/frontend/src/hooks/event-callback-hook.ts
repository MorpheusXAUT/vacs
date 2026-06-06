import {useCallback, useLayoutEffect, useRef} from "preact/hooks";

export function useEventCallback<TArgs extends unknown[], TResult>(
    fn: (...args: TArgs) => TResult,
): (...args: TArgs) => TResult {
    const fnRef = useRef(fn);

    useLayoutEffect(() => {
        fnRef.current = fn;
    }, [fn]);

    return useCallback((...args: TArgs): TResult => {
        return fnRef.current(...args);
    }, []);
}
