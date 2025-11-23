import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";
import {filterAndSortClients, StationsConfig, StationsProfileConfig} from "../types/stations.ts";
import {invokeStrict} from "../error.ts";

type ConnectionState = "connecting" | "connected" | "disconnected";

type SignalingState = {
    connectionState: ConnectionState;
    displayName: string;
    frequency: string;
    allClients: ClientInfo[]; // all available clients, including those filtered out by stations config
    clients: ClientInfo[]; // list of clients to be displayed in UI, pre-processed by stations config and priority/sorting
    stationsConfig: StationsConfig | undefined;
    activeStationsProfileConfig: string;
    setConnectionState: (state: ConnectionState) => void;
    setClientInfo: (info: Omit<ClientInfo, "id">) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfo;
    removeClient: (cid: string) => void;
    setStationsConfig: (config: StationsConfig) => void;
    setActiveStationsProfileConfig: (profile: string) => void;
    getActiveStationsProfileConfig: () => StationsProfileConfig | undefined;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connectionState: "disconnected",
    displayName: "",
    frequency: "",
    allClients: [],
    clients: [],
    stationsConfig: undefined,
    activeStationsProfileConfig: "Default",
    setConnectionState: (connectionState) => set({connectionState}),
    setClientInfo: (info) => set({displayName: info.displayName, frequency: info.frequency}),
    setClients: (clients) => {
        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig())
        });
    },
    addClient: (client) => {
        const clients = get().allClients.filter(c => c.id !== client.id);
        clients.push(client);
        set({
            allClients: clients,
            clients: filterAndSortClients(clients, get().getActiveStationsProfileConfig())
        });
    },
    getClientInfo: (cid) => {
        const client = get().allClients.find(c => c.id === cid);
        if (client === undefined) {
            return {id: cid, displayName: cid, frequency: ""};
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
        set({
            stationsConfig: config,
            clients: filterAndSortClients(get().allClients, get().getActiveStationsProfileConfig()),
        });
    },
    setActiveStationsProfileConfig: (profile) => {
        set({activeStationsProfileConfig: profile});
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