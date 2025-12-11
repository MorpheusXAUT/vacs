import {useSignalingStore} from "../stores/signaling-store.ts";
import DAKey from "./ui/DAKey.tsx";
import Button from "./ui/Button.tsx";
import {navigate} from "wouter/use-browser-location";
import {ClientInfoWithAlias} from "../types/client-info.ts";
import {useCallStore} from "../stores/call-store.ts";

type DAKeyAreaProps = {
    filter: string;
};

function DAKeyArea({filter}: DAKeyAreaProps) {
    const clients = useSignalingStore(state => state.clients);
    const grouping = useSignalingStore(state => state.getActiveStationsProfileConfig()?.grouping);

    const getGroups = (clients: ClientInfoWithAlias[], slice: number, prefix = "") => {
        const groups = [
            ...clients
                .filter(
                    client =>
                        client.displayName.startsWith(prefix) && client.displayName.includes("_"),
                )
                .reduce<
                    Set<string>
                >((acc, val) => acc.add(val.displayName.split("_")[0].slice(0, slice)), new Set([])),
        ];

        if (
            clients.find(client => !client.displayName.includes("_")) !== undefined &&
            prefix === ""
        ) {
            groups.push("OTHER");
        }

        return groups;
    };

    const renderClients = (clients: ClientInfoWithAlias[]) => {
        return clients.map((client, idx) => <DAKey key={idx} client={client} />);
    };

    const renderGroups = (groups: string[]) => {
        return groups.map((group, idx) => <DANavKey key={idx} group={group} />);
    };

    const renderKeys = () => {
        if (filter === "OTHER") {
            return renderClients(clients.filter(client => !client.displayName.includes("_")));
        }

        switch (grouping) {
            case "Fir":
            case "Icao": {
                if (filter !== "") {
                    return renderClients(
                        clients.filter(client => client.displayName.startsWith(filter)),
                    );
                }

                const slice = grouping === "Fir" ? 2 : 4;
                return renderGroups(getGroups(clients, slice));
            }
            case "FirAndIcao": {
                if (filter === "") {
                    return renderGroups(getGroups(clients, 2));
                } else if (filter.length === 2) {
                    return renderGroups(getGroups(clients, 4, filter));
                }
                return renderClients(
                    clients.filter(client => client.displayName.startsWith(filter)),
                );
            }
            case undefined:
            case "None":
            default:
                return renderClients(clients);
        }
    };

    return (
        <div className="grid grid-rows-6 grid-flow-col h-full py-3 px-2 gap-3 overflow-x-auto overflow-y-hidden">
            {renderKeys()}
        </div>
    );
}

function DANavKey({group}: {group: string}) {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);

    const isClientInGroup = (client: string) => {
        return client.startsWith(group) || (group === "OTHER" && !client.includes("_"));
    };

    const isCalling = incomingCalls.some(peer => isClientInGroup(peer.displayName));
    const beingCalled =
        callDisplay?.type === "outgoing" && isClientInGroup(callDisplay.peer.displayName);
    const inCall =
        callDisplay?.type === "accepted" && isClientInGroup(callDisplay.peer.displayName);
    const isRejected =
        callDisplay?.type === "rejected" && isClientInGroup(callDisplay.peer.displayName);
    const isError = callDisplay?.type === "error" && isClientInGroup(callDisplay.peer.displayName);

    return (
        <Button
            color={
                inCall
                    ? "green"
                    : (isCalling || isRejected) && blink
                      ? "green"
                      : isError && blink
                        ? "red"
                        : "gray"
            }
            highlight={beingCalled || isRejected ? "green" : undefined}
            className="w-25 h-full rounded !leading-4.5 p-1.5"
            onClick={() => navigate(group)}
        >
            <p className="w-full truncate leading-3.5" title={group}>
                {group}
                <br />
                ...
            </p>
        </Button>
    );
}

export default DAKeyArea;
