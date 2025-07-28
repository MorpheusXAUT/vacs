import Button from "./ui/Button.tsx";
import {useSignalingStore} from "../stores/signaling-store.ts";

function DAKeyArea() {
    const clients = useSignalingStore(state => state.clients);

    return (
        <div className="flex flex-col flex-wrap h-full overflow-hidden py-3 px-2 gap-3 relative">
            {clients.map(client =>
                <Button color="gray" className="w-25 h-[calc((100%-3.75rem)/6)] rounded !leading-4.5">{client.displayName}<br/>{client.id}</Button>
            )}
            {/*<div className="w-5 h-5 bg-red-500 absolute top-[50%]"></div> 320-340<br/>E2<br/>EC*/}
        </div>
    );
}

export default DAKeyArea;