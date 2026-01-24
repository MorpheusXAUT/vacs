import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useClientsStore} from "../stores/clients-store.ts";
import {ClientInfo, SessionInfo} from "../types/client-info.ts";
import {useCallStore} from "../stores/call-store.ts";
import {useErrorOverlayStore} from "../stores/error-overlay-store.ts";
import {useCallListStore} from "../stores/call-list-store.ts";
import {useConnectionStore} from "../stores/connection-store.ts";
import {PositionId} from "../types/generic.ts";
import {useProfileStore} from "../stores/profile-store.ts";
import {StationChange, StationInfo} from "../types/station.ts";
import {useStationsStore} from "../stores/stations-store.ts";

export function setupSignalingListeners() {
    const {setClients, addClient, getClientInfo, removeClient} = useClientsStore.getState();
    const {setStations, addStationChanges} = useStationsStore.getState();
    const {
        addIncomingCall,
        removePeer,
        rejectPeer,
        acceptCall,
        reset: resetCallStore,
    } = useCallStore.getState().actions;
    const {open: openErrorOverlay} = useErrorOverlayStore.getState();
    const {addCall: addCallToCallList, clearCallList} = useCallListStore.getState().actions;
    const {setConnectionState, setConnectionInfo, setPositionsToSelect} =
        useConnectionStore.getState();
    const {setProfile} = useProfileStore.getState();

    const unlistenFns: Promise<UnlistenFn>[] = [];

    const init = () => {
        unlistenFns.push(
            listen<SessionInfo>("signaling:connected", event => {
                setConnectionState("connected");
                setConnectionInfo(event.payload.client);
                if (
                    event.payload.profile.type === "changed" &&
                    event.payload.profile.activeProfile !== undefined &&
                    event.payload.profile.activeProfile.profile !== undefined
                ) {
                    setProfile(event.payload.profile.activeProfile.profile);
                }
            }),
            listen("signaling:reconnecting", () => {
                setConnectionState("connecting");
            }),
            listen("signaling:disconnected", () => {
                setConnectionState("disconnected");
                setConnectionInfo({displayName: "", positionId: undefined, frequency: ""});
                setClients([]);
                setStations([]);
                resetCallStore();
                clearCallList();
                setProfile(undefined);
            }),
            listen<PositionId[]>("signaling:ambiguous-position", event => {
                setConnectionState("connecting");
                setPositionsToSelect(event.payload);
            }),
            listen<StationInfo[]>("signaling:station-list", event => {
                setStations(event.payload);
            }),
            listen<StationChange[]>("signaling:station-changes", event => {
                addStationChanges(event.payload);
            }),
            listen<ClientInfo[]>("signaling:client-list", event => {
                setClients(event.payload);
            }),
            listen<ClientInfo>("signaling:client-connected", event => {
                addClient(event.payload);
            }),
            listen<string>("signaling:client-disconnected", event => {
                removeClient(event.payload);
                removePeer(event.payload);
            }),
            listen<string>("signaling:call-invite", event => {
                addIncomingCall(getClientInfo(event.payload));
            }),
            listen<string>("signaling:call-accept", event => {
                acceptCall(getClientInfo(event.payload));
            }),
            listen<string>("signaling:call-end", event => {
                removePeer(event.payload, true);
            }),
            listen<string>("signaling:force-call-end", event => {
                removePeer(event.payload);
            }),
            listen<string>("signaling:call-reject", event => {
                rejectPeer(event.payload);
            }),
            listen<string>("signaling:peer-not-found", event => {
                removeClient(event.payload);
                removePeer(event.payload);
                openErrorOverlay(
                    "Peer not found",
                    `Cannot find peer with CID ${event.payload}`,
                    false,
                    5000,
                );
            }),
            listen<{incoming: boolean; peerId: string}>("signaling:add-to-call-list", event => {
                const clientInfo = getClientInfo(event.payload.peerId);
                addCallToCallList({
                    type: event.payload.incoming ? "IN" : "OUT",
                    time: new Date().toLocaleString("de-AT", {
                        hour: "2-digit",
                        minute: "2-digit",
                        timeZone: "UTC",
                    }),
                    name: clientInfo.displayName,
                    number: event.payload.peerId,
                });
            }),
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    };
}
