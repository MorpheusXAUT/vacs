import {ClientInfo, ClientInfoWithAlias} from "../types/client-info.ts";
import {create} from "zustand/react";
import {filterAndSortClients, StationsConfig, StationsProfileConfig} from "../types/stations.ts";
import {invokeStrict} from "../error.ts";

type ConnectionState = "connecting" | "connected" | "disconnected";

type SignalingState = {
    connectionState: ConnectionState;
    displayName: string;
    alias: string | undefined;
    frequency: string;
    allClients: ClientInfoWithAlias[]; // all available clients, including those filtered out by stations config
    clients: ClientInfoWithAlias[]; // list of clients to be displayed in UI, pre-processed by stations config and priority/sorting
    stationsConfig: StationsConfig | undefined;
    activeStationsProfileConfig: string;
    setConnectionState: (state: ConnectionState) => void;
    setClientInfo: (info: Omit<ClientInfo, "id">) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfoWithAlias;
    removeClient: (cid: string) => void;
    setStationsConfig: (config: StationsConfig) => void;
    setActiveStationsProfileConfig: (profile: string) => void;
    getActiveStationsProfileConfig: () => StationsProfileConfig | undefined;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connectionState: "disconnected",
    displayName: "",
    alias: undefined,
    frequency: "",
    allClients: [],
    clients: [],
    stationsConfig: undefined,
    activeStationsProfileConfig: "Default",
    setConnectionState: (connectionState) => set({connectionState}),
    setClientInfo: (info) => {
        set({
            displayName: info.displayName,
            alias: get().getActiveStationsProfileConfig()?.aliases?.[info.frequency],
            frequency: info.frequency,
        })
    },
    setClients: (clients) => {
        const aliases = get().getActiveStationsProfileConfig()?.aliases ?? {};

        clients = [
            ...clients,
            {id: "", frequency: "132.600", displayName: "LOVV_CTR"},
            {id: "", frequency: "124.400", displayName: "LOVV_I_CTR"},
            {id: "", frequency: "134.675", displayName: "LOWW_PBSPECIALSNOWFLAKE_APP"},
            {id: "", frequency: "119.400", displayName: "LOWW_TWR"}
        ]

        const clientsWithAliases = clients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency]
        }));

        set({
            allClients: clientsWithAliases,
            clients: filterAndSortClients(clientsWithAliases, get().getActiveStationsProfileConfig())
        });
    },
    addClient: (client) => {
        const clients = get().allClients.filter(c => c.id !== client.id);

        clients.push({
            ...client,
            alias: get().getActiveStationsProfileConfig()?.aliases?.[client.frequency]
        });

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig())
        });
    },
    getClientInfo: (cid) => {
        const client = get().allClients.find(c => c.id === cid);
        if (client === undefined) {
            return {id: cid, displayName: cid, alias: undefined, frequency: ""};
        }
        return client;
    },
    removeClient: (cid) => {
        set({
            allClients: get().allClients.filter(client => client.id !== cid),
            clients: get().clients.filter(client => client.id !== cid),
        });
    },
    setStationsConfig: (config) => {
        set({stationsConfig: config});

        const aliases = get().getActiveStationsProfileConfig()?.aliases ?? {};
        const clients = get().allClients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency]
        }));

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig()),
        });
    },
    setActiveStationsProfileConfig: (profile) => {
        set({activeStationsProfileConfig: profile});

        const newProfile = get().getActiveStationsProfileConfig();
        const aliases = newProfile?.aliases ?? {};
        const clients = get().allClients.map<ClientInfoWithAlias>(client => ({
            ...client,
            alias: aliases[client.frequency]
        }));

        set({
            allClients: clients,
            clients: filterAndSortClients(clients, newProfile),
        });
    },
    getActiveStationsProfileConfig: () => {
        const config = get().stationsConfig;
        if (config === undefined) return undefined;
        return config.profiles[get().activeStationsProfileConfig] ?? config.profiles["Default"];
    }
}));

export const fetchStationsConfig = async () => {
    try {
        const config = await invokeStrict<StationsConfig>("signaling_get_stations_config");

        useSignalingStore.getState().setStationsConfig(config);
    } catch {
    }
};