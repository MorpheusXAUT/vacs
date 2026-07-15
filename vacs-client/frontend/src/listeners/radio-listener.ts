import {listen, UnlistenFn} from "../transport";
import {RadioState} from "../types/radio.ts";
import {useRadioStore} from "../stores/radio-store.ts";
import {invokeStrict} from "../error.ts";

export function setupRadioListener() {
    const {setRadioState} = useRadioStore.getState();

    const unlistenFns: Promise<UnlistenFn>[] = [];

    const init = () => {
        unlistenFns.push(
            listen<RadioState>("radio:state", event => {
                console.log("Radio state:", event.payload);
                setRadioState(event.payload);
            }),
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    };
}

export async function fetchRadioState() {
    try {
        const state = await invokeStrict<RadioState>("radio_get_state");
        useRadioStore.getState().setRadioState(state);
    } catch {}
}
