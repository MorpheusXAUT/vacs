import Button from "../components/ui/Button.tsx";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {invokeSafe} from "../error.ts";

function ConnectPage() {
    const handleConnectClick = useAsyncDebounce(async () => {
        await invokeSafe("signaling_connect");
    });

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <Button
                color="green"
                className="w-auto px-10 py-3 text-xl"
                onClick={handleConnectClick}
            >
                Connect
            </Button>
        </div>
    );
}

export default ConnectPage;