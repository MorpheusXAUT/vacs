import Button from "./ui/Button.tsx";
import {useDialPadInput} from "../hooks/dial-pad-hook.ts";
import {clsx} from "clsx";
import {useCallStore} from "../stores/call-store.ts";
import {invokeStrict} from "../error.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {TargetedEvent} from "preact";
import {useSignalingStore} from "../stores/signaling-store.ts";

const DIAL_BUTTONS: {digit: string, chars: string}[] = [
    { digit: "1", chars: ""},
    { digit: "2", chars: "ABC"},
    { digit: "3", chars: "DEF"},
    { digit: "4", chars: "GHI"},
    { digit: "5", chars: "JKL"},
    { digit: "6", chars: "MNO"},
    { digit: "7", chars: "PQRS"},
    { digit: "8", chars: "TUV"},
    { digit: "9", chars: "WXYZ"},
    { digit: "*", chars: ""},
    { digit: "0", chars: ""},
    { digit: "#", chars: ""},
];

function DialPad() {
    const {dialInput, setDialInput, handleDialClick, clearLastChar, clearAll} = useDialPadInput();
    const isDialInputEmpty = dialInput === "";
    const isConnected = useSignalingStore(state => state.connectionState === "connected");
    const callDisplay = useCallStore(state => state.callDisplay);
    const {setOutgoingCall, removePeer} = useCallStore(state => state.actions);

    const handleChange = (event: TargetedEvent<HTMLInputElement>) => {
        if (event.target instanceof HTMLInputElement) {
            const rawValue = event.target.value;

            const sanitized = rawValue.toUpperCase().replace(/[^A-Z0-9*#]/g, "").slice(0, 7);
            event.target.value = sanitized;

            setDialInput(sanitized);
        }
    };

    const handleCallOnClick = useAsyncDebounce(async () => {
        if (isDialInputEmpty || callDisplay !== undefined) return;
        try {
            setOutgoingCall({id: dialInput, displayName: dialInput, frequency: ""});
            await invokeStrict("signaling_start_call", {peerId: dialInput});
        } catch {
            removePeer(dialInput);
        }
    });

    return (
        <div className="w-full flex flex-row [&_button]:h-15 [&_button]:shrink-0 [&_button]:rounded py-3">
            <div className="flex flex-col gap-3 px-4 pt-[calc(4.5rem-1px)]">
                <Button color="blue" disabled={false}>IA</Button>
                <Button color="cyan">
                    <p>ATS<br/>MFC</p>
                </Button>
                <Button color="cyan" disabled={true}/>
                <Button color="cyan" disabled={true}/>
                <Button color="gray">Redial</Button>
            </div>
            <div>
                <input
                    type="text"
                    className={clsx(
                        "w-[21.75rem] h-15 px-3 rounded border border-gray-700 bg-slate-200 text-3xl font-semibold overflow-auto leading-14 mb-3",
                        "focus:border-red-500 focus:outline-none"
                    )}
                    onChange={handleChange}
                    value={dialInput}
                />
                <div className="grid grid-cols-3 gap-3 [&>button]:w-27 mb-3">
                    {DIAL_BUTTONS.map(({digit, chars}, idx) =>
                        <Button key={idx} color="gray" className="text-lg" onClick={() => handleDialClick(digit, chars)}>
                            {chars !== "" ? (<p>{digit}<br/>{chars}</p>) : (<p>{digit}</p>)}
                        </Button>
                    )}
                </div>
                <Button color="gray" className="w-full text-xl" title={!isConnected ? "Disconnected" : undefined} disabled={isDialInputEmpty || !isConnected} onClick={handleCallOnClick}>Call</Button>
            </div>
            <div className="flex flex-col gap-3 px-4">
                <Button color="gray" disabled={isDialInputEmpty}
                    onClick={clearLastChar}
                >
                    ⟵
                </Button>
                <Button color="gray" disabled={isDialInputEmpty}
                    onClick={clearAll}
                >
                    <p>Clear<br/>All</p>
                </Button>
            </div>
        </div>
    );
}

export default DialPad;