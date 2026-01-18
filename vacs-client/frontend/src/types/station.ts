import {PositionId, StationId} from "./generic.ts";

export type StationInfo = {
    id: StationId;
    own: boolean;
};

export type StationChange = {
    online?: {
        stationId: StationId;
        positionId: PositionId;
    };
    handoff?: {
        stationId: StationId;
        fromPositionId: PositionId;
        toPositionId: PositionId;
    };
    offline?: {
        stationId: StationId;
    };
};
