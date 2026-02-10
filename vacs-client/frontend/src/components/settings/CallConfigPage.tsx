import {CloseButton} from "../../pages/SettingsPage.tsx";
import Checkbox from "../ui/Checkbox.tsx";
import {useSettingsStore} from "../../stores/settings-store.ts";
import {invokeStrict} from "../../error.ts";
import {CallConfig} from "../../types/settings.ts";

function CallConfigPage() {
    const callConfig = useSettingsStore(state => state.callConfig);
    const setCallConfig = useSettingsStore(state => state.setCallConfig);

    return (
        <div className="absolute top-0 z-10 h-full w-2/5 bg-blue-700 border-t-0 px-2 pb-2 flex flex-col">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Call Config</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col overflow-y-auto">
                <div className="w-full py-3 px-4 grow border-b-2 border-zinc-200 flex flex-col gap-3">
                    <CallConfigEntry
                        label="Highlight incoming target"
                        name="display-call-target"
                        property="highlightIncomingCallTarget"
                        callConfig={callConfig}
                        setCallConfig={setCallConfig}
                    />
                    <CallConfigEntry
                        label="Enable priority calls"
                        name="enable-priority-calls"
                        property="enablePriorityCalls"
                        callConfig={callConfig}
                        setCallConfig={setCallConfig}
                    />
                    <CallConfigEntry
                        label="Play call start sound"
                        name="enable-call-start-sound"
                        property="enableCallStartSound"
                        callConfig={callConfig}
                        setCallConfig={setCallConfig}
                    />
                    <CallConfigEntry
                        label="Play call end sound"
                        name="enable-call-end-sound"
                        property="enableCallEndSound"
                        callConfig={callConfig}
                        setCallConfig={setCallConfig}
                    />
                </div>
                <div className="h-20 w-full shrink-0 flex flex-row gap-2 justify-end p-2 [&>button]:px-1 [&>button]:shrink-0 overflow-x-auto scrollbar-hide">
                    <CloseButton />
                </div>
            </div>
        </div>
    );
}

type CallConfigEntryProps = {
    label: string;
    name: string;
    callConfig: CallConfig;
    setCallConfig: (config: CallConfig) => void;
    property: keyof CallConfig;
};

function CallConfigEntry(props: CallConfigEntryProps) {
    return (
        <div className="w-full flex justify-between items-center">
            <label htmlFor={props.name}>{props.label}</label>
            <Checkbox
                name={props.name}
                checked={props.callConfig[props.property]}
                muted={
                    (props.property === "enableCallEndSound" ||
                        props.property === "enableCallStartSound") &&
                    !props.callConfig[props.property]
                }
                onChange={async event => {
                    const next = event.currentTarget.checked;
                    const config = {
                        ...props.callConfig,
                        [props.property]: next,
                    };

                    try {
                        await invokeStrict("app_set_call_config", {callConfig: config});
                        props.setCallConfig(config);
                    } catch {
                        props.setCallConfig({
                            ...props.callConfig,
                            [props.property]: !next,
                        });
                    }
                }}
            />
        </div>
    );
}

export default CallConfigPage;
