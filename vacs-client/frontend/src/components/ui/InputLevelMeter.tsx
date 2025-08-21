import {useEffect, useState} from "preact/hooks";
import {listen} from "@tauri-apps/api/event";
import {InputLevel} from "../../types/audio.ts";
import {clsx} from "clsx";
import {invokeSafe} from "../../error.ts";

function InputLevelMeter() {
    const [activated, setActivated] = useState<boolean>(false);
    const [level, setLevel] = useState<number>(0);

    useEffect(() => {
        const unlisten = listen<InputLevel>("audio:input-level", (event) => {
            setLevel(event.payload.norm);
        })

        return () => {
            unlisten.then(f => f());
        };
    }, []);

    const handleOnClick = () => {
        if (activated) {
            void invokeSafe("audio_stop_input_level_meter");
            setActivated(false);
            setLevel(0);
        } else {
            setActivated(true);
            void invokeSafe("audio_start_input_level_meter");
        }
    };

    return (
        <div className="w-4 h-full shrink-0 pb-2 pt-24">
            <div
                className={clsx(
                    "relative w-full h-full border-2 rounded cursor-pointer",
                    activated ? "border-blue-700" : "border-gray-500"
                )}
                onClick={handleOnClick}
            >
                <div className="absolute bg-[rgba(0,0,0,0.3)] w-full"
                     style={{height: `${100 - level * 100}%`}}></div>
                <div className="bg-red-500 w-full h-[5%]"></div>
                <div className="bg-yellow-400 w-full h-[10%]"></div>
                <div className="bg-green-500 w-full h-[15%]"></div>
                <div className="bg-green-600 w-full h-[55%]"></div>
                <div className="bg-blue-600 w-full h-[15%]"></div>
            </div>
        </div>
    );
}

export default InputLevelMeter;