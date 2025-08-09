import {useSignalingStore} from "../stores/signaling-store.ts";
import DAKey from "./ui/DAKey.tsx";

function DAKeyArea() {
    const clients = useSignalingStore(state => state.clients);

    return (
        <div className="flex flex-col flex-wrap h-full overflow-hidden py-3 px-2 gap-3">
            {clients.map(client =>
                <DAKey client={client}/>
            )}
        </div>
    );
}

export default DAKeyArea;