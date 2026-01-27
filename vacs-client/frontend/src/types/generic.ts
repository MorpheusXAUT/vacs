declare const BrandSymbol: unique symbol;

type Brand<T, B> = T & {readonly [BrandSymbol]: B};

export type ClientId = Brand<string, "ClientId">;
export type PositionId = Brand<string, "PositionId">;
export type StationId = Brand<string, "StationId">;
export type ProfileId = Brand<string, "ProfileId">;
export type CallId = Brand<string, "CallId">;
