import {PositionId} from "./generic.ts";
import {SessionProfile} from "./profile.ts";

export type ClientInfo = {
    id: string;
    positionId: PositionId | undefined;
    displayName: string;
    frequency: string;
};

export type SessionInfo = {
    client: ClientInfo;
    profile: SessionProfile;
};

export function splitDisplayName(client: ClientInfo): [string, string] {
    const parts = client.displayName.split("_");

    if (parts.length <= 1) {
        return [parts[0], ""];
    }

    return [parts.slice(0, parts.length - 1).join(" "), parts[parts.length - 1]];
}
