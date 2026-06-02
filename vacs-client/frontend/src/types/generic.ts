declare const BrandSymbol: unique symbol;

type Brand<T, B> = T & {readonly [BrandSymbol]: B};

export type ClientId = Brand<string, "ClientId">;
export type PositionId = Brand<string, "PositionId">;
export type StationId = Brand<string, "StationId">;
export type ProfileId = Brand<string, "ProfileId">;
export type CallId = Brand<string, "CallId">;

export type StateSetter<T> = (value: T | ((prev: T) => T)) => void;

export function createSetter<T>(set: any) {
    return <K extends keyof T>(
            key: K,
            cmp?: (next: T[K], prev?: T[K]) => void,
        ): StateSetter<T[K]> =>
        value =>
            set((prev: T) => {
                const next =
                    typeof value === "function"
                        ? (value as (prev: T[K]) => T[K])(prev[key])
                        : value;
                cmp?.(next, prev[key]);
                return {
                    [key]: next,
                };
            });
}
