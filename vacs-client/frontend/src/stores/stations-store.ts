import {create} from "zustand/react";
import {StationChange, StationInfo} from "../types/station.ts";
import {useConnectionStore} from "./connection-store.ts";
import {StationId} from "../types/generic.ts";

type StationsState = {
    stations: Map<StationId, boolean>; // boolean => own
    defaultSource: StationId | undefined;
    temporarySource: StationId | undefined;
    setStations: (stations: StationInfo[]) => void;
    addStationChanges: (changes: StationChange[]) => void;
    setDefaultSource: (source: StationId | undefined) => void;
    setTemporarySource: (source: StationId | undefined) => void;
    reset: () => void;
};

export const useStationsStore = create<StationsState>()((set, get) => ({
    stations: new Map(),
    defaultSource: undefined,
    temporarySource: undefined,
    setStations: stations => set({stations: new Map(stations.map(s => [s.id, s.own]))}),
    addStationChanges: changes => {
        const stations = new Map(get().stations);
        const ownPositionId = useConnectionStore.getState().info.positionId;

        for (const change of changes) {
            if (change.online !== undefined) {
                stations.set(change.online.stationId, change.online.positionId === ownPositionId);
            } else if (change.handoff !== undefined) {
                stations.set(
                    change.handoff.stationId,
                    change.handoff.toPositionId === ownPositionId,
                );
            } else if (change.offline !== undefined) {
                stations.delete(change.offline.stationId);
            }
        }

        set({stations});
    },
    setDefaultSource: source => set({defaultSource: source}),
    setTemporarySource: source => set({temporarySource: source}),
    reset: () => set({stations: new Map(), defaultSource: undefined, temporarySource: undefined}),
}));
