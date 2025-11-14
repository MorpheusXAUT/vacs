import {useSignalingStore} from "../stores/signaling-store.ts";
import DAKey from "./ui/DAKey.tsx";

function DAKeyArea() {
    const clients = useSignalingStore(state => state.clients);

    return (
        <div className="grid grid-rows-6 grid-flow-col h-full py-3 px-2 gap-3 overflow-x-auto overflow-y-hidden">
            {clients.map((client, idx) =>
                <DAKey key={idx} client={client}/>
            )}
        </div>
    );
}

export default DAKeyArea;