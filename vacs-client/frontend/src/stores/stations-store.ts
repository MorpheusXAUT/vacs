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

export const useStationsStore = create<StationsState>()((set, get, store) => ({
    stations: new Map(),
    defaultSource: undefined,
    temporarySource: undefined,
    setStations: stationsList => {
        const stations = new Map(stationsList.map(s => [s.id, s.own]));

        const [defaultSource, temporarySource] = checkStationSourcesAreOwn(stations, get);

        set({stations, defaultSource, temporarySource});
    },
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

        const [defaultSource, temporarySource] = checkStationSourcesAreOwn(stations, get);

        set({stations, defaultSource, temporarySource});
    },
    setDefaultSource: source => set({defaultSource: source}),
    setTemporarySource: source => set({temporarySource: source}),
    reset: () => set(store.getInitialState()),
}));

function checkStationSourcesAreOwn(
    stations: Map<StationId, boolean>,
    get: () => StationsState,
): [StationId | undefined, StationId | undefined] {
    let defaultSource = get().defaultSource;
    if (defaultSource !== undefined && !stations.get(defaultSource)) {
        defaultSource = undefined;
    }

    let temporarySource = get().temporarySource;
    if (temporarySource !== undefined && !stations.get(temporarySource)) {
        temporarySource = undefined;
    }

    return [defaultSource, temporarySource];
}
