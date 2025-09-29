import {useAuthStore} from "../stores/auth-store.ts";
import "../styles/info-grid.css";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {useCallStore} from "../stores/call-store.ts";
import {useUpdateStore} from "../stores/update-store.ts";
import {navigate} from "wouter/use-browser-location";
import {invokeSafe} from "../error.ts";

function InfoGrid() {
    const cid = useAuthStore(state => state.cid);
    const clientInfo = useSignalingStore(state => `${state.displayName}${state.frequency !== "" ? `(${state.frequency})` : ""}`);
    const callErrorReason = useCallStore(state => state.callDisplay?.errorReason);
    const currentVersion = useUpdateStore(state => state.currentVersion);
    const newVersion = useUpdateStore(state => state.newVersion);

    const versionText = `${newVersion === undefined ? "Version: " : ""}v${currentVersion}${newVersion ? ` - UPDATE AVAILABLE (v${newVersion})` : ""}`;

    const handleVersionClick = () => {
        void invokeSafe("audio_play_ui_click");
        navigate("/settings")
    };

    return (
        <div className="grid grid-rows-2 w-full h-full" style={{gridTemplateColumns: "25% 32.5% 42.5%"}}>
            <div className="info-grid-cell" title={cid}>{cid}</div>
            <div className="info-grid-cell cursor-pointer" title={versionText}
                 onClick={handleVersionClick}>{currentVersion !== "" ? versionText : ""}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell" title={clientInfo}>{clientInfo}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell uppercase" title={callErrorReason}>{callErrorReason}</div>
        </div>
    );
}

export default InfoGrid;