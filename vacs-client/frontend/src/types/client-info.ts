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

        // Sort clients with no station types last
        if (aStationType.length === 0 && bStationType.length > 0) {
            return 1;
        } else if (aStationType.length > 0 && bStationType.length === 0) {
            return -1;
        }

        const stationType = aStationType.localeCompare(bStationType);
        return stationType !== 0 ? stationType : aStationName.localeCompare(bStationName);
    });
}