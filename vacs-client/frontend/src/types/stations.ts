import {ClientInfoWithAlias, splitDisplayName} from "./client-info.ts";

export type StationsConfig = {
    selectedProfile: string;
    profiles: StationsConfigProfiles;
}

export type StationsConfigProfiles = Record<string, StationsProfileConfig>;

export type StationsProfileConfig = {
    include: string[];
    exclude: string[];
    priority: string[];
    aliases: Record<string, string>;
    frequencies: FrequencyDisplayMode;
    grouping: StationsGroupMode;
}

export type FrequencyDisplayMode = "ShowAll" | "HideAliased" | "HideAll";
export type StationsGroupMode = "None" | "Fir" | "FirAndAirport" | "Airport";

function globToRegex(pattern: string): RegExp {
    const escaped = pattern
        .replace(/[.+^${}()|[\]\\]/g, '\\$&') // Escape regex special chars except * and ?
        .replace(/\*/g, '.*')                 // * matches any characters
        .replace(/\?/g, '.');                 // ? matches single character

    return new RegExp(`^${escaped}$`, "i");
}

function matchesAnyPattern(callsign: string, patterns: string[]): boolean {
    if (patterns.length === 0) return false;
    return patterns.some(pattern => globToRegex(pattern).test(callsign));
}


function findFirstMatchIndex(callsign: string, patterns: string[]): number {
    return patterns.findIndex(pattern => globToRegex(pattern).test(callsign));
}

function filterClients(clients: ClientInfoWithAlias[], profile: StationsProfileConfig | undefined): ClientInfoWithAlias[] {
    if (!profile) return clients;

    return clients.filter(client => {
        if (matchesAnyPattern(client.displayName, profile.exclude)) return false;
        if (profile.include.length === 0) return true;
        return matchesAnyPattern(client.displayName, profile.include);
    })
}

function sortClients(clients: ClientInfoWithAlias[], profile: StationsProfileConfig | undefined): ClientInfoWithAlias[] {
    if (!profile) return clients;

    return clients.sort((a, b) => {
        const aPriorityIndex = findFirstMatchIndex(a.alias ?? a.displayName, profile.priority);
        const bPriorityIndex = findFirstMatchIndex(b.alias ?? b.displayName, profile.priority);

        // 1. Sort by priority bucket (lower index = higher priority)
        const aEffectivePriority = aPriorityIndex === -1 ? Number.MAX_SAFE_INTEGER : aPriorityIndex;
        const bEffectivePriority = bPriorityIndex === -1 ? Number.MAX_SAFE_INTEGER : bPriorityIndex;

        if (aEffectivePriority !== bEffectivePriority) {
            return aEffectivePriority - bEffectivePriority;
        }

        const [aStationName, aStationType] = splitDisplayName(a);
        const [bStationName, bStationType] = splitDisplayName(b);

        // 2. Sort non-prioritized station types before clients without any station type
        if (aStationType.length === 0 && bStationType.length > 0) {
            return 1;
        } else if (aStationType.length > 0 && bStationType.length === 0) {
            return -1;
        }

        // 3. Sort by station type alphabetically
        const stationType = aStationType.localeCompare(bStationType);

        // 4. Sort by station name alphabetically
        return stationType !== 0 ? stationType : aStationName.localeCompare(bStationName);
    })
}

export function filterAndSortClients(clients: ClientInfoWithAlias[], profile: StationsProfileConfig | undefined): ClientInfoWithAlias[] {
    const filtered = filterClients(clients, profile);
    return sortClients(filtered, profile);
}