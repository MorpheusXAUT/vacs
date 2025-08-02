import Button from "./ui/Button.tsx";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {invokeSafe, invokeStrict} from "../error.ts";
import {navigate} from "wouter/use-browser-location";

function DAKeyArea() {
    const clients = useSignalingStore(state => state.clients);

    const handleDAClick = useAsyncDebounce(async (clientId: string) => {
        await invokeSafe("signaling_da_key_click", {clientId: clientId});
    });

    return (
        <div className="flex flex-col flex-wrap h-full overflow-hidden py-3 px-2 gap-3 relative">
            {clients.map(client =>
                <Button color="gray" className="w-25 h-[calc((100%-3.75rem)/6)] rounded !leading-4.5"
                        onClick={() => handleDAClick(client.id)}><p>{client.displayName}<br/>{client.id}</p></Button>
            )}
            {/*<div className="w-5 h-5 bg-red-500 absolute top-[50%]"></div> 320-340<br/>E2<br/>EC*/}
        </div>
    );
}

export default DAKeyArea;