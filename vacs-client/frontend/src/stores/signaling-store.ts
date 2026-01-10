import {ClientInfo, ClientInfoWithAlias} from "../types/client-info.ts";
import {create} from "zustand/react";
import {
    filterAndSortClients,
    StationsConfigProfiles,
    StationsConfig,
    StationsProfileConfig,
} from "../types/stations.ts";
import {invokeStrict} from "../error.ts";

type SignalingState = {
    allClients: ClientInfoWithAlias[]; // all available clients, including those filtered out by stations config
    clients: ClientInfoWithAlias[]; // list of clients to be displayed in UI, pre-processed by stations config and priority/sorting
    stationsConfigProfiles: StationsConfigProfiles;
    activeStationsProfileConfig: string;
    terminateOverlayOpen: boolean;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfoWithAlias;
    removeClient: (cid: string) => void;
    setStationsConfig: (config: StationsConfig) => void;
    setActiveStationsProfileConfig: (profile: string) => void;
    getActiveStationsProfileConfig: () => StationsProfileConfig | undefined;
    setTerminateOverlayOpen: (open: boolean) => void;
};

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    allClients: [],
    clients: [],
    stationsConfigProfiles: {},
    activeStationsProfileConfig: "Default",
    terminateOverlayOpen: false,
    setClients: clients => {
        const aliases = get().getActiveStationsProfileConfig()?.aliases ?? {};

        const clientsWithAliases = clients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency],
        }));

        set({
            allClients: clientsWithAliases,
            clients: filterAndSortClients(
                clientsWithAliases,
                get().getActiveStationsProfileConfig(),
            ),
        });
    },
    addClient: client => {
        const clients = get().allClients.filter(c => c.id !== client.id);

        clients.push({
            ...client,
            alias: get().getActiveStationsProfileConfig()?.aliases?.[client.frequency],
        });

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig()),
        });
    },
    getClientInfo: cid => {
        const client = get().allClients.find(c => c.id === cid);
        if (client === undefined) {
            return {id: cid, displayName: cid, alias: undefined, frequency: ""};
        }
        return client;
    },
    removeClient: cid => {
        set({
            allClients: get().allClients.filter(client => client.id !== cid),
            clients: get().clients.filter(client => client.id !== cid),
        });
    },
    setStationsConfig: config => {
        set({
            activeStationsProfileConfig: config.selectedProfile,
            stationsConfigProfiles: config.profiles,
        });

        const aliases = get().getActiveStationsProfileConfig()?.aliases ?? {};
        const clients = get().allClients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency],
        }));

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig()),
        });
    },
    setActiveStationsProfileConfig: profile => {
        set({activeStationsProfileConfig: profile});

        const newProfile = get().getActiveStationsProfileConfig();
        const aliases = newProfile?.aliases ?? {};
        const clients = get().allClients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency],
        }));

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, newProfile),
        });
    },
    getActiveStationsProfileConfig: () => {
        const profiles = get().stationsConfigProfiles;
        if (profiles === undefined) return undefined;
        return profiles[get().activeStationsProfileConfig] ?? profiles["Default"];
    },
    setTerminateOverlayOpen: open => set({terminateOverlayOpen: open}),
}));

export const fetchStationsConfig = async () => {
    try {
        const config = await invokeStrict<StationsConfig>("signaling_get_stations_config");

        useSignalingStore.getState().setStationsConfig(config);
    } catch {}
};
