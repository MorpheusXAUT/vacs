import {useState} from "preact/hooks";
import Button, {DisabledButtonColors} from "./button.tsx";
import {clsx} from "clsx";

function IncomingList() {
    const [showIncoming, setShowIncoming] = useState(false);
    const [fixIncomingActive, _setFixIncomingActive] = useState(false);

    return (
        <div className="flex flex-col-reverse gap-3 pt-3 pr-[1px] overflow-y-auto" style={{scrollbarWidth: "none"}}>
            <div className="w-full border rounded min-h-16"></div>
            {showIncoming ?
                <Button color="green" className={clsx("min-h-16", fixIncomingActive && DisabledButtonColors["green"])}
                               >Test</Button>
                :
                <div className="w-full border rounded min-h-16" onClick={() => setShowIncoming(true)}></div>}
            <div className="w-full border rounded min-h-16"></div>
            <div className="w-full border rounded min-h-16"></div>
            <div className="w-full border rounded min-h-16"></div>
            <div className="w-full border rounded min-h-16"></div>
        </div>
    );
}

export default IncomingList;