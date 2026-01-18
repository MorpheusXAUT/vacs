import {create} from "zustand/react";
import {StationChange, StationInfo} from "../types/station.ts";
import {useConnectionStore} from "./connection-store.ts";
import {StationId} from "../types/generic.ts";

type StationsState = {
    stations: Map<StationId, boolean>; // boolean => own
    setStations: (stations: StationInfo[]) => void;
    addStationChanges: (changes: StationChange[]) => void;
};

export const useStationsStore = create<StationsState>()((set, get) => ({
    stations: new Map(),
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
}));
