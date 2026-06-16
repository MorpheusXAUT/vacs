import {invokeStrict} from "../../error.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import {ALL_CPL_MODES, isCplMode} from "../../types/settings.ts";
import Hint from "../Hint.tsx";
import Select from "../ui/Select.tsx";

const CPL_MODE_OPTIONS = ALL_CPL_MODES.map(mode => ({value: mode, text: mode}));

function CplModeSettings() {
    const cplMode = useSettingsStore(state => state.cplMode);
    const setCplMode = useSettingsStore(state => state.setCplMode);

    const handleOnChange = useAsyncDebounce(async (value: string) => {
        if (!isCplMode(value)) return;
        const previousMode = cplMode;

        setCplMode(value);

        try {
            await invokeStrict("app_set_cpl_mode", {cplMode: value});
        } catch {
            setCplMode(previousMode);
        }
    });

    return (
        <>
            <div className="w-full flex flex-row gap-2 items-center justify-center pt-1">
                <p className="text-center uppercase font-semibold">Couple Mode</p>
                <Hint hint="Original: Click CPL to enter Couple Mode, click a frequency to couple it, then click CPL again to exit. Double-clicking CPL while outside Couple Mode couples all TX-enabled frequencies at once. | Fast: Click FAST CPL to immediately couple all TX-enabled frequencies at once." />
            </div>
            <div className="w-full py-3 px-4 border-b-2 border-zinc-200 flex flex-col gap-3">
                <Select
                    name="cpl-mode"
                    className="mb-1"
                    options={CPL_MODE_OPTIONS}
                    selected={cplMode}
                    onChange={handleOnChange}
                />
            </div>
        </>
    );
}

export default CplModeSettings;
