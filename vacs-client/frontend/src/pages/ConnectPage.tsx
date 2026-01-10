import Button from "../components/ui/Button.tsx";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {clsx} from "clsx";
import {useConnectionStore, connect} from "../stores/connection-store.ts";

function ConnectPage() {
    const connecting = useConnectionStore(state => state.connectionState === "connecting");

    const handleConnectClick = useAsyncDebounce(async () => {
        if (connecting) return;
        await connect();
    });

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <Button
                color="green"
                className={clsx("w-50 px-5 py-3 text-xl")}
                onClick={handleConnectClick}
            >
                {!connecting ? "Connect" : "Connecting..."}
            </Button>
        </div>
    );
}

export default ConnectPage;
