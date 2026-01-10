import {useAsyncDebounceState} from "../../hooks/debounce-hook.ts";
import {invokeStrict} from "../../error.ts";
import Button from "../ui/Button.tsx";
import {clsx} from "clsx";
import {connect, useConnectionStore} from "../../stores/connection-store.ts";

function ConnectionTerminateOverlay() {
    const visible = useConnectionStore(state => state.terminateOverlayVisible);
    const setVisible = useConnectionStore(state => state.setTerminateOverlayVisible);

    const [handleTerminateClick, terminateLoading] = useAsyncDebounceState(async () => {
        try {
            await invokeStrict("signaling_terminate");
            setVisible(false);
            await connect();
        } catch {}
    });

    return visible ? (
        <div className="z-50 absolute top-0 left-0 w-full h-full flex justify-center items-center bg-[rgba(0,0,0,0.5)]">
            <div className="bg-gray-300 border-4 border-t-red-500 border-l-red-500 border-b-red-700 border-r-red-700 rounded w-100 py-2">
                <p className="w-full text-center text-lg font-semibold wrap-break-word">
                    Already connected
                </p>
                <p className="w-full text-center wrap-break-word mb-2">
                    Your CID is already connected to the server. Do you wish to terminate the other
                    client and connect anyways?
                </p>
                <div
                    className={clsx(
                        "w-full flex flex-row gap-2 justify-center items-center mb-2",
                        terminateLoading && "brightness-90 [&>button]:cursor-not-allowed",
                    )}
                >
                    <Button
                        color="red"
                        className="px-3 py-1"
                        onClick={() => setVisible(false)}
                        disabled={terminateLoading}
                    >
                        No
                    </Button>
                    <Button
                        color="green"
                        className="px-3 py-1"
                        onClick={handleTerminateClick}
                        disabled={terminateLoading}
                    >
                        Yes
                    </Button>
                </div>
                {terminateLoading && (
                    <p className="w-full text-center font-semibold">Terminating...</p>
                )}
            </div>
        </div>
    ) : (
        <></>
    );
}

export default ConnectionTerminateOverlay;
