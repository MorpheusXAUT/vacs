import {useAuthStore} from "../stores/auth-store.ts";
import "../styles/info-grid.css";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {useCallStore} from "../stores/call-store.ts";

function InfoGrid() {
    const cid = useAuthStore(state => state.cid);
    const displayName = useSignalingStore(state => state.displayName);
    const callErrorReason = useCallStore(state => state.callDisplay?.errorReason ?? "");

    return (
        <div className="grid grid-rows-2 w-full h-full" style={{ gridTemplateColumns: "25% 32.5% 42.5%" }}>
            <div className="info-grid-cell">{cid}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell">{displayName}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell uppercase">{callErrorReason}</div>
        </div>
    );
}

export default InfoGrid;