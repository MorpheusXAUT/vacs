// import {useState} from "preact/hooks";
import {clsx} from "clsx";
import {useSignalingStore} from "../../stores/signaling-store.ts";

type Status = "green" | "yellow" | "red" | "gray";

const StatusColors: Record<Status, string> = {
    green: "bg-green-600 border-green-700",
    yellow: "bg-yellow-500 border-yellow-600",
    red: "bg-red-400 border-red-700",
    gray: "bg-gray-400 border-gray-600"
};

function StatusIndicator() {
    //const [status, _setStatus] = useState<Status>("yellow");
    const connected = useSignalingStore(state => state.connected);
    const status = connected ? "green" : "red";

    return (
        <div className={clsx("h-full aspect-square rounded-full border", StatusColors[status])}></div>
    );
}

export default StatusIndicator;