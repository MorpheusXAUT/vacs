import Button from "../components/ui/Button.tsx";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {clsx} from "clsx";
import {connect, useSignalingStore} from "../stores/signaling-store.ts";

function ConnectPage() {
    const connecting = useSignalingStore(state => state.connectionState === "connecting");

    const handleConnectClick = useAsyncDebounce(connect);

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <Button
                color="green"
                className={clsx(
                    "w-50 px-5 py-3 text-xl",
                    connecting && "brightness-90 cursor-not-allowed",
                )}
                onClick={handleConnectClick}
            >
                {!connecting ? "Connect" : "Connecting..."}
            </Button>
        </div>
    );
}

export default ConnectPage;
