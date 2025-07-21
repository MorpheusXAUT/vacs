import {useEffect, useState} from "preact/hooks";
import {listen} from "@tauri-apps/api/event";
import {invoke} from "@tauri-apps/api/core";

type InfoGridProps = {
    displayName: string
};

function InfoGrid(props: InfoGridProps) {
    const [cid, setCid] = useState<string>("");

    useEffect(() => {
        const unlistenCid = listen<string>("vatsim-cid", (event) => {
            setCid(event.payload);
        });

        void checkAuthentication();

        return () => {
            unlistenCid.then(f => f());
        };
    }, []);

    async function checkAuthentication() {
        try {
            await invoke("check_auth_session");
        } catch (e) {
            // TODO handle error
            console.error(e);
        }
    }

    return (
        <div className="grid grid-rows-2 w-full h-full" style={{ gridTemplateColumns: "25% 32.5% 42.5%" }}>
            <div className="info-grid-cell">{cid}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell">{props.displayName}</div>
            <div className="info-grid-cell"></div>
            <div className="info-grid-cell"></div>
        </div>
    );
}

export default InfoGrid;