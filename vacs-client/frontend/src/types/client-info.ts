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