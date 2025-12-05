const ALL_TRANSMIT_MODES = ["VoiceActivation", "PushToTalk", "PushToMute", "RadioIntegration"] as const;
export type TransmitMode = typeof ALL_TRANSMIT_MODES[number];

export function isTransmitMode(value: string): value is TransmitMode {
    return ALL_TRANSMIT_MODES.includes(value as TransmitMode);
}

const ALL_RADIO_INTEGRATIONS = ["AudioForVatsim", "TrackAudio"] as const;
export type RadioIntegration = typeof ALL_RADIO_INTEGRATIONS[number];

export function isRadioIntegration(value: string): value is RadioIntegration {
    return ALL_RADIO_INTEGRATIONS.includes(value as RadioIntegration);
}

export type TransmitConfig = {
    mode: TransmitMode;
    pushToTalk: string | null;
    pushToMute: string | null;
    radioPushToTalk: string | null;
}

export type TransmitConfigWithLabels = TransmitConfig & {
    pushToTalkLabel: string | null;
    pushToMuteLabel: string | null;
    radioPushToTalkLabel: string | null;
}

export type RadioConfig = {
    integration: RadioIntegration;
    audioForVatsim: AudioForVatsimRadioConfig | null;
    trackAudio: TrackAudioRadioConfig | null;
}

export type RadioConfigWithLabels = RadioConfig & {
    audioForVatsim: AudioForVatsimRadioConfigLabels | null;
}

export type AudioForVatsimRadioConfig = {
    emit: string | null;
}

export type AudioForVatsimRadioConfigLabels = {
    emitLabel: string | null;
}

export type TrackAudioRadioConfig = {
    endpoint: string | null;
}

export async function withLabels(config: TransmitConfig): Promise<TransmitConfigWithLabels> {
    return {
        ...config,
        pushToTalkLabel: config.pushToTalk && await codeToLabel(config.pushToTalk),
        pushToMuteLabel: config.pushToMute && await codeToLabel(config.pushToMute),
        radioPushToTalkLabel: config.radioPushToTalk && await codeToLabel(config.radioPushToTalk),
    };
}

export async function withRadioLabels(config: RadioConfig): Promise<RadioConfigWithLabels> {
    return {
        ...config,
        audioForVatsim: config.audioForVatsim && {
            ...config.audioForVatsim,
            emitLabel: config.audioForVatsim.emit && await codeToLabel(config.audioForVatsim.emit)
        },
    }
}

export async function codeToLabel(code: string): Promise<string> {
    const keyboard = (navigator as {
        keyboard?: {
            getLayoutMap: () => Promise<{ get: (value: string) => string }>
        }
    }).keyboard;

    if (keyboard?.getLayoutMap) {
        try {
            const map = await keyboard.getLayoutMap();
            const label = map.get(code);
            if (label) return label.toUpperCase();
        } catch {
        }
    }

    return prettyFormatKeyCode(code);
}

export function prettyFormatKeyCode(keyCode: string): string {
    switch (keyCode) {
        case "Backquote":
            return "`";
        case "Backslash":
            return "\\";
        case "BracketLeft":
            return "[";
        case "BracketRight":
            return "]";
        case "Comma":
            return ",";
        case "Digit0":
            return "0";
        case "Digit1":
            return "1";
        case "Digit2":
            return "2";
        case "Digit3":
            return "3";
        case "Digit4":
            return "4";
        case "Digit5":
            return "5";
        case "Digit6":
            return "6";
        case "Digit7":
            return "7";
        case "Digit8":
            return "8";
        case "Digit9":
            return "9";
        case "Equal":
            return "=";
        case "IntlBackslash":
            return "|";
        case "KeyA":
            return "A";
        case "KeyB":
            return "B";
        case "KeyC":
            return "C";
        case "KeyD":
            return "D";
        case "KeyE":
            return "E";
        case "KeyF":
            return "F";
        case "KeyG":
            return "G";
        case "KeyH":
            return "H";
        case "KeyI":
            return "I";
        case "KeyJ":
            return "J";
        case "KeyK":
            return "K";
        case "KeyL":
            return "L";
        case "KeyM":
            return "M";
        case "KeyN":
            return "N";
        case "KeyO":
            return "O";
        case "KeyP":
            return "P";
        case "KeyQ":
            return "Q";
        case "KeyR":
            return "R";
        case "KeyS":
            return "S";
        case "KeyT":
            return "T";
        case "KeyU":
            return "U";
        case "KeyV":
            return "V";
        case "KeyW":
            return "W";
        case "KeyX":
            return "X";
        case "KeyY":
            return "Y";
        case "KeyZ":
            return "Z";
        case "Minus":
            return "-";
        case "Period":
            return ".";
        case "Quote":
            return "\"";
        case "Semicolon":
            return ";";
        case "Slash":
            return "/";
        case "Numpad0":
            return "Num0";
        case "Numpad1":
            return "Num1";
        case "Numpad2":
            return "Num2";
        case "Numpad3":
            return "Num3";
        case "Numpad4":
            return "Num4";
        case "Numpad5":
            return "Num5";
        case "Numpad6":
            return "Num6";
        case "Numpad7":
            return "Num7";
        case "Numpad8":
            return "Num8";
        case "Numpad9":
            return "Num9";
        case "NumpadAdd":
            return "Num+";
        case "NumpadBackspace":
            return "NumBackspace";
        case "NumpadClear":
            return "NumClear";
        case "NumpadClearEntry":
            return "NumClearEntry";
        case "NumpadComma":
            return "Num,";
        case "NumpadDecimal":
            return "Num.";
        case "NumpadDivide":
            return "Num/";
        case "NumpadEnter":
            return "NumEnter";
        case "NumpadEqual":
            return "Num=";
        case "NumpadHash":
            return "Num#";
        case "NumpadMemoryAdd":
            return "NumMemoryAdd";
        case "NumpadMemoryClear":
            return "NumMemoryClear";
        case "NumpadMemoryRecall":
            return "NumMemoryRecall";
        case "NumpadMemoryStore":
            return "NumMemoryStore";
        case "NumpadMemorySubtract":
            return "NumMemorySubtract";
        case "NumpadMultiply":
            return "Num*";
        case "NumpadParenLeft":
            return "Num(";
        case "NumpadParenRight":
            return "Num)";
        case "NumpadStar":
            return "Num*";
        case "NumpadSubtract":
            return "Num-";
        default:
            return keyCode;
    }
}