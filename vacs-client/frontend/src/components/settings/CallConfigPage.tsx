import {CloseButton} from "../../pages/SettingsPage.tsx";
import Checkbox from "../ui/Checkbox.tsx";
import {useSettingsStore} from "../../stores/settings-store.ts";
import {invokeStrict} from "../../error.ts";

function CallConfigPage() {
    const callConfig = useSettingsStore(state => state.callConfig);
    const setCallConfig = useSettingsStore(state => state.setCallConfig);

    return (
        <div className="absolute top-0 z-10 h-full w-2/5 bg-blue-700 border-t-0 px-2 pb-2 flex flex-col">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Call Config</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col overflow-y-auto">
                <div className="w-full py-3 px-4 grow border-b-2 border-zinc-200 flex flex-col gap-3">
                    <div className="w-full flex justify-between items-center">
                        <label htmlFor="display-call-target">Highlight incoming target</label>
                        <Checkbox
                            name="display-call-target"
                            checked={callConfig.highlightIncomingCallTarget}
                            onChange={async e => {
                                const next = e.currentTarget.checked;
                                const config = {
                                    ...callConfig,
                                    highlightIncomingCallTarget: next,
                                };

                                try {
                                    await invokeStrict("app_set_call_config", {callConfig: config});
                                    setCallConfig(config);
                                } catch {
                                    setCallConfig({
                                        ...callConfig,
                                        highlightIncomingCallTarget: !next,
                                    });
                                }
                            }}
                        />
                    </div>
                    <div className="w-full flex justify-between items-center">
                        <label htmlFor="disable-priority-calls">Disable priority calls</label>
                        <Checkbox
                            name="disable-priority-calls"
                            checked={callConfig.disablePriorityCalls}
                            onChange={async e => {
                                const next = e.currentTarget.checked;
                                const config = {
                                    ...callConfig,
                                    disablePriorityCalls: next,
                                };

                                try {
                                    await invokeStrict("app_set_call_config", {callConfig: config});
                                    setCallConfig(config);
                                } catch {
                                    setCallConfig({
                                        ...callConfig,
                                        disablePriorityCalls: !next,
                                    });
                                }
                            }}
                        />
                    </div>
                </div>
                <div className="h-20 w-full shrink-0 flex flex-row gap-2 justify-end p-2 [&>button]:px-1 [&>button]:shrink-0 overflow-x-auto scrollbar-hide">
                    <CloseButton />
                </div>
            </div>
        </div>
    );
}

export default CallConfigPage;
