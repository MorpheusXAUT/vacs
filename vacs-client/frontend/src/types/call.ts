import {CallId, ClientId, PositionId, StationId} from "./generic.ts";

export type CallSource = {
    clientId: ClientId;
    positionId?: PositionId;
    stationId?: StationId;
};

export type CallTarget = {
    client?: ClientId;
    position?: PositionId;
    station?: StationId;
};

export type Call = {
    callId: CallId;
    source: CallSource;
    target: CallTarget;
};
