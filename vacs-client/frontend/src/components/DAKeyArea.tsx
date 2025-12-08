import {useSignalingStore} from "../stores/signaling-store.ts";
import DAKey from "./ui/DAKey.tsx";
import Button from "./ui/Button.tsx";
import {navigate} from "wouter/use-browser-location";
import {ClientInfoWithAlias} from "../types/client-info.ts";

type DAKeyAreaProps = {
    filter: string;
};

function DAKeyArea({filter}: DAKeyAreaProps) {
    const clients = useSignalingStore(state => state.clients);
    const grouping = useSignalingStore(state => state.getActiveStationsProfileConfig()?.grouping);

    return (
        <div className="grid grid-rows-6 grid-flow-col h-full py-3 px-2 gap-3 overflow-x-auto overflow-y-hidden">
            {(() => {
                switch (grouping) {
                    case "Fir":
                    case "Airport": {
                        if (filter !== "") {
                            return renderClients(clients.filter(client => client.displayName.startsWith(filter)));
                        }

                        const slice = grouping === "Fir" ? 2 : 4;
                        return renderGroups(getGroups(clients, slice));
                    }
                    case "FirAndAirport": {
                        if (filter === "") {
                            return renderGroups(getGroups(clients, 2));
                        } else if (filter.length === 2) {
                            return renderGroups(getGroups(clients, 4, filter));
                        }
                        return renderClients(clients.filter(client => client.displayName.startsWith(filter)));
                    }
                    case undefined:
                    case "None":
                    default:
                        return renderClients(clients);
                }
            })()}
        </div>
    );
}

function getGroups(clients: ClientInfoWithAlias[], slice: number, prefix = "") {
    return [
        ...clients
            .filter(client => client.displayName.startsWith(prefix))
            .reduce<Set<string>>((acc, val) =>
                acc.add(val.displayName.slice(0, slice)
                ), new Set([]))
    ];
}

function renderClients(clients: ClientInfoWithAlias[]) {
    return clients.map((client, idx) =>
        <DAKey key={idx} client={client}/>
    );
}

function renderGroups(groups: string[]) {
    return groups.map((group, idx) =>
        <Button
            key={idx}
            color="gray"
            className="w-25 h-full rounded !leading-4.5 p-1.5"
            onClick={() => navigate(group)}
        >
            <p className="w-full truncate" title={group}>{group}</p>
        </Button>
    );
}

export default DAKeyArea;