import {useRef, useState} from "preact/hooks";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {InputLevel} from "../../types/audio.ts";
import {clsx} from "clsx";
import {invokeSafe} from "../../error.ts";
import {useCallStore} from "../../stores/call-store.ts";

function InputLevelMeter() {
    const isCallActive = useCallStore(state => state.callDisplay?.type === "accepted");
    const [unlistenFn, setUnlistenFn] = useState<Promise<UnlistenFn> | undefined>();
    const [level, setLevel] = useState<InputLevel | undefined>();
    const unlistenStopFnRef = useRef<Promise<UnlistenFn> | undefined>();

    const handleOnClick = async () => {
        if (isCallActive) return; // Cannot start input level meter while call is active

        void invokeSafe("audio_play_ui_click");

        if (unlistenStopFnRef.current) {
            (await unlistenStopFnRef.current)();
        }

        if (unlistenFn !== undefined) {
            await invokeSafe("audio_stop_input_level_meter");

            (await unlistenFn)();
            setUnlistenFn(undefined);
            setLevel(undefined);
        } else {
            const unlisten = listen<InputLevel>("audio:input-level", (event) => {
                setLevel(event.payload);
            })

            const unlistenStop = listen("audio:stop-input-level-meter", async () => {
                (await unlisten)();
                setUnlistenFn(undefined);
                setLevel(undefined);
            });

            setUnlistenFn(unlisten);
            unlistenStopFnRef.current = unlistenStop;
            void invokeSafe("audio_start_input_level_meter");
        }
    };


    return (
        <div className="w-4 h-full shrink-0 pb-2 pt-24">
            <div
                className={clsx(
                    "relative w-full h-full border-2 rounded",
                    unlistenFn === undefined ? "border-gray-500" : level?.clipping ? "border-red-700" : "border-blue-700",
                    isCallActive ? "cursor-not-allowed" : "cursor-pointer",
                )}
                onClick={handleOnClick}
            >
                <div className="absolute bg-[rgba(0,0,0,0.5)] w-full"
                     style={{height: `${100 - (level?.norm ?? 0) * 100}%`}}></div>
                <div className="bg-red-500 w-full h-[5%]"></div>
                <div className="bg-yellow-400 w-full h-[10%]"></div>
                <div className="bg-green-500 w-full h-[20%]"></div>
                <div className="bg-green-600 w-full h-[40%]"></div>
                <div className="bg-blue-600 w-full h-[25%]"></div>
            </div>
        </div>
    );
}

export default InputLevelMeter;