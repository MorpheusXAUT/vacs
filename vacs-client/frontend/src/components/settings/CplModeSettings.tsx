import {invokeStrict} from "../../error.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import Hint from "../Hint.tsx";
import Select from "../ui/Select.tsx";
import {ALL_CPL_MODES, isCplMode, RadioConfig} from "../../types/transmit.ts";

const CPL_MODE_OPTIONS = ALL_CPL_MODES.map(mode => ({value: mode, text: mode}));

function CplModeSettings() {
    const cplMode = useSettingsStore(state => state.radioConfig?.cplMode ?? "Original");
    const setRadioConfig = useSettingsStore(state => state.setRadioConfig);

    const handleOnChange = useAsyncDebounce(async (value: string) => {
        const radioConfig = useSettingsStore.getState().radioConfig;
        if (!isCplMode(value) || radioConfig === undefined) return;

        const newRadioConfig: RadioConfig = {...radioConfig, cplMode: value};

        try {
            await invokeStrict("keybinds_set_radio_config", {radioConfig: newRadioConfig});
            setRadioConfig({...radioConfig, cplMode: value});
        } catch {}
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
