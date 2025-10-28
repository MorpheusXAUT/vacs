import Button from "../components/ui/Button.tsx";
import {useAsyncDebounce, useAsyncDebounceState} from "../hooks/debounce-hook.ts";
import {invoke} from "@tauri-apps/api/core";
import {invokeStrict, isError, openErrorOverlayFromUnknown} from "../error.ts";
import {useState} from "preact/hooks";
import {clsx} from "clsx";
import {useSignalingStore} from "../stores/signaling-store.ts";

function ConnectPage() {
    const connecting = useSignalingStore(state => state.connectionState === "connecting");
    const setConnectionState = useSignalingStore(state => state.setConnectionState);
    const [terminateDialogOpen, setTerminateDialogOpen] = useState<boolean>(false);

    const handleConnectClick = useAsyncDebounce(async () => {
        setConnectionState("connecting");
        try {
            await invoke("signaling_connect");
        } catch (e) {
            setConnectionState("disconnected");
            if (isError(e) && (e.message === "Login failed: Another client with your CID is already connected." || e.message === "Already connected")) {
                setTerminateDialogOpen(true);
                return;
            }
            openErrorOverlayFromUnknown(e);
        }
    });

    const [handleTerminateClick, terminateLoading] = useAsyncDebounceState(async () => {
        try {
            await invokeStrict("signaling_terminate");
            setTerminateDialogOpen(false);
            void handleConnectClick();
        } catch {
        }
    });

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <Button
                color="green"
                className={clsx("w-50 px-5 py-3 text-xl", connecting && "brightness-90 cursor-not-allowed")}
                onClick={handleConnectClick}
            >
                {!connecting ? "Connect" : "Connecting..."}
            </Button>

            <div
                style={{display: terminateDialogOpen ? "flex" : "none"}}
                className="z-50 absolute top-0 left-0 w-full h-full justify-center items-center bg-[rgba(0,0,0,0.5)]"
            >
                <div
                    className="bg-gray-300 border-4 border-t-red-500 border-l-red-500 border-b-red-700 border-r-red-700 rounded w-100 py-2">
                    <p className="w-full text-center text-lg font-semibold wrap-break-word">Already connected</p>
                    <p className="w-full text-center wrap-break-word mb-2">
                        Your CID is already connected to the server.
                        Do you wish to terminate the other client and connect anyways?
                    </p>
                    <div
                        className={clsx("w-full flex flex-row gap-2 justify-center items-center mb-2", terminateLoading && "brightness-90 [&>button]:cursor-not-allowed")}>
                        <Button color="red" className="px-3 py-1" onClick={() => setTerminateDialogOpen(false)}
                                disabled={terminateLoading}>No</Button>
                        <Button color="green" className="px-3 py-1" onClick={handleTerminateClick}
                                disabled={terminateLoading}>Yes</Button>
                    </div>
                    {terminateLoading && <p className="w-full text-center font-semibold">Terminating...</p>}
                </div>
            </div>
        </div>
    );
}

export default ConnectPage;