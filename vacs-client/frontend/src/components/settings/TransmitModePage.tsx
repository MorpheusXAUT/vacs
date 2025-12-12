import TransmitModeSettings from "./TransmitModeSettings.tsx";
import {CloseButton} from "../../pages/SettingsPage.tsx";

function TransmitModePage() {
    return (
        <div className="absolute top-0 z-10 h-full w-2/3 bg-blue-700 border-t-0 px-2 pb-2 flex flex-col">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">
                Transmit Config
            </p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col">
                <div className="w-full grow border-b-2 border-zinc-200 flex flex-col">
                    <TransmitModeSettings />
                </div>
                <div className="h-20 w-full flex flex-row gap-2 justify-end p-2 [&>button]:px-1 [&>button]:shrink-0 overflow-x-auto scrollbar-hide">
                    <CloseButton />
                </div>
            </div>
        </div>
    );
}

export default TransmitModePage;
