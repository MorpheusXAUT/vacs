export type ClientInfo = {
    id: string;
    displayName: string;
    frequency: string;
};

export function splitDisplayName(displayName: string): [string, string] {
    const parts = displayName.split("_");

    if (parts.length <= 1) {
        return [parts[0], ""];
    }

    return [parts.slice(0, parts.length - 1).join(" "), parts[parts.length - 1]];
}

export function sortClients(clients: ClientInfo[]): ClientInfo[] {
    return clients.sort((a, b) => {
        const [aStationName, aStationType] = splitDisplayName(a.displayName);
        const [bStationName, bStationType] = splitDisplayName(b.displayName);

        const aStationRank = getStationTypeRank(aStationType.toUpperCase());
        const bStationRank = getStationTypeRank(bStationType.toUpperCase());

        // First sort by custom sort order for station types
        if (aStationRank !== bStationRank) {
            return aStationRank - bStationRank;
        }

        const stationType = aStationType.localeCompare(bStationType);
        // Secondly, sort by station type (alphabetical) if they have the same sort order
        // Otherwise, sort by station name (alphabetical)
        return stationType !== 0 ? stationType : aStationName.localeCompare(bStationName);
    });
}

const STATION_TYPE_ORDER: Record<string, number> = {
    "CTR": 0,
    "APP": 1,
    "TWR": 2,
    "GND": 3,
    "DEL": 4,
};

function getStationTypeRank(stationType: string): number {
    // Clients without a station type are sorted to the end of the list
    if (stationType.length === 0) {
        return Number.MAX_SAFE_INTEGER;
    }

    // Clients with an unknown station type are sorted to the end of the list, but before the ones without a station type
    return STATION_TYPE_ORDER[stationType] ?? Object.keys(STATION_TYPE_ORDER).length + 1;
}