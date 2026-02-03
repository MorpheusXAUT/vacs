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

export function callSourceToTarget(source: CallSource): CallTarget {
    if (source.stationId !== undefined) {
        return {station: source.stationId};
    } else if (source.positionId !== undefined) {
        return {position: source.positionId};
    }
    return {client: source.clientId};
}
