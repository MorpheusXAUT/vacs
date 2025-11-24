export type ClientInfo = {
    id: string;
    displayName: string;
    frequency: string;
};

export type ClientInfoWithAlias = ClientInfo & {
    alias: string | undefined
};

export function splitDisplayName(client: ClientInfoWithAlias): [string, string] {
    const parts = (client.alias ?? client.displayName).split("_");

    if (parts.length <= 1) {
        return [parts[0], ""];
    }

    return [parts.slice(0, parts.length - 1).join(" "), parts[parts.length - 1]];
}